#![deny(warnings)]
extern crate log;
extern crate pretty_env_logger;
extern crate warp;
extern crate reqwest;
extern crate bytes;
use std::time::Duration;
use std::env;

use std::sync::{Arc, RwLock};
use warp::{Filter, http::Response};
use timed_string::TimedString;

type Store = Arc<RwLock<TimedString>>;

mod timed_string;


/// Provides a RESTful web server managing some Todos.
///
/// API will be:
///
/// - `GET /buses`: return a JSON list of buses.
fn main() {
    pretty_env_logger::init();

    // Turn our state into a Filter so we can combine it easily later
    let bus_cache = TimedString::new("http://developer.itsmarta.com/BRDRestService/RestBusRealTimeService/GetAllBus",
                                     Duration::from_secs(10));
    let bus_cache = Arc::new(RwLock::new(bus_cache));
    let bus_cache = warp::any().map(move || bus_cache.clone());

    let train_route = format!(
        "http://developer.itsmarta.com/RealtimeTrain/RestServiceNextTrain/GetRealtimeArrivals?apikey={}",
        env::var("MARTA_TRAIN_API_KEY").expect("MARTA_TRAIN_API_KEY must be set")
    );
    let trains_cache = TimedString::new(train_route, Duration::from_secs(10));
    let trains_cache = Arc::new(RwLock::new(trains_cache));
    let trains_cache = warp::any().map(move || trains_cache.clone());

    let buses = warp::path("buses");
    let buses_index = buses.and(warp::path::end());

    // `GET /buses`
    let list = warp::get2()
        .and(buses_index)
        .and(bus_cache.clone())
        .map(cache_visit);

    let trains = warp::path("trains");
    let trains_index = trains.and(warp::path::end());

    // `GET /trains`
    let list2 = warp::get2()
        .and(trains_index)
        .and(trains_cache.clone())
        .map(cache_visit);
    // View access logs by setting `RUST_LOG=buses`.
    let routes = list.with(warp::log("buses"))
        .or(list2.with(warp::log("trains")));

    // optionally binding it to SSL on port 443
    if let Some(ssl_key) = env::var_os("SSL_KEY") {
        let ssl_cert = env::var("SSL_CERT").expect("SSL_CERT must be set if SSL_KEY is set");
        warp::serve(routes.clone())
            .tls(ssl_cert, ssl_key)
            .run(([0, 0, 0, 0], 443));
    } else {
        // Start up the server...
        let port: u16 = env::var("PORT")
            .unwrap_or("3030".to_string())
            .parse()
            .expect("PORT must be set to a valid numeric port");
        warp::serve(routes.clone())
            .run(([0, 0, 0, 0], port));
    }
}


// read from cache if it's still valid, otherwise hit the URL + write
fn cache_visit(cache: Store) -> impl warp::Reply {
    {
        // base case: cache still valid, only need a read lock.
        // should be very fast
        let timed_string = cache.read().unwrap();
        if timed_string.is_valid() {
            return Response::new(timed_string.text.clone());
        }
    }

    // acquire write lock
    let mut timed_string = cache.write().unwrap();
    // check if someone else acquired the write lock while we were blocked
    // and updated the cache already, so just return result
    if timed_string.is_valid() {
        return Response::new(timed_string.text.clone());
    }

    // otherwise we have to update the cache, slowest case.
    timed_string.refresh();
    Response::new(timed_string.text.clone())
}
