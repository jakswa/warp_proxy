#![deny(warnings)]
extern crate log;
extern crate pretty_env_logger;
extern crate warp;
extern crate reqwest;
extern crate bytes;
extern crate listenfd;
extern crate tokio;

use std::time::Duration;
use std::env;
use bytes::Bytes;

use std::sync::{Arc, RwLock};
use warp::Filter;
use warp::http::{Response,StatusCode};
use listenfd::ListenFd;
use cached_bytes::CachedBytes;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::reactor::Handle;

type Store = Arc<RwLock<CachedBytes>>;

mod cached_bytes;


/// Provides a RESTful web server managing some Todos.
///
/// API will be:
///
/// - `GET /buses`: return a JSON list of buses.
fn main() {
    pretty_env_logger::init();

    // Turn our state into a Filter so we can combine it easily later
    let bus_cache = CachedBytes::new("http://developer.itsmarta.com/BRDRestService/RestBusRealTimeService/GetAllBus",
                                     Duration::from_secs(10));
    let bus_cache = Arc::new(RwLock::new(bus_cache));
    let bus_cache = warp::any().map(move || bus_cache.clone());

    let train_route = format!(
        "http://developer.itsmarta.com/RealtimeTrain/RestServiceNextTrain/GetRealtimeArrivals?apikey={}",
        env::var("MARTA_TRAIN_API_KEY").unwrap_or("please_set_api_key".to_string())
    );
    let trains_cache = CachedBytes::new(train_route, Duration::from_secs(10));
    let trains_cache = Arc::new(RwLock::new(trains_cache));
    let trains_cache = warp::any().map(move || trains_cache.clone());

    let buses = warp::path("buses");
    let buses_index = buses.and(warp::path::end());

    // `GET /buses`
    let list = warp::get2()
        .and(buses_index)
        .and(bus_cache.clone())
        .map(cache_visit)
        .with(warp::reply::with::header("Access-Control-Allow-Origin", "*"))
        .with(warp::reply::with::header("Wombats", "always"));

    let trains = warp::path("trains");
    let trains_index = trains.and(warp::path::end());

    // `GET /trains`
    let list2 = warp::get2()
        .and(trains_index)
        .and(trains_cache.clone())
        .map(cache_visit)
        .with(warp::reply::with::header("Access-Control-Allow-Origin", "*"));
    // View access logs by setting `RUST_LOG=buses`.
    let routes = list.with(warp::log("buses"))
        .or(list2.with(warp::log("trains")));


    let server = warp::serve(routes.clone());
    // if let Some(ssl_key) = env::var_os("SSL_KEY") {
    //     let ssl_cert = env::var("SSL_CERT").unwrap();
    //     server = server.tls(ssl_cert, ssl_key);
    // }

    let mut listenfd = ListenFd::from_env();
    match listenfd.take_tcp_listener(0) {
        Ok(Some(listener)) => {
            if let Ok(tokio_listener) = TokioTcpListener::from_std(listener, &Handle::default()) {
                server.run_incoming(tokio_listener.incoming());
            }
        },
        _ => {

            let port: u16 = env::var("PORT")
                .unwrap_or("3030".to_string())
                .parse().unwrap();

            match env::var_os("SSL_KEY") {
                Some(_) => server.run(([0, 0, 0, 0], 443)),
                _ => server.run(([0, 0, 0, 0], port))
            }
        }
    }
    ()
}


// read from cache store if it's still valid, otherwise hit the URL + write
fn cache_visit(store: Store) -> impl warp::Reply {
    {
        // base case: store still valid, only need a read lock.
        // should be very fast
        let cache = store.read().unwrap();
        if cache.is_valid() {
            return Response::new(cache.bytes())
        }
    }

    // acquire write lock
    let mut cache = store.write().unwrap();
    // check if someone else acquired the write lock while we were blocked
    // and updated the store already, so just return result
    if cache.is_valid() {
        return Response::new(cache.bytes());
    }

    // otherwise we have to update the store, slowest case.
    match cache.refresh() {
        Ok(_) => Response::new(cache.bytes()),
        Err(e) => {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Bytes::from(format!("{}", e)))
                .unwrap()
        }
    }
}
