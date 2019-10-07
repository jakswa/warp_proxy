#![deny(warnings)]
extern crate log;
extern crate pretty_env_logger;
extern crate warp;
extern crate reqwest;
extern crate bytes;
use std::time::{Duration, Instant};
use std::env;

use std::sync::{Arc, RwLock};
//use warp::{http::StatusCode, Filter};
use warp::{Filter, http::Response};
use bytes::Bytes;

type Store = Arc<RwLock<TimedString>>;

struct TimedString {
    time: Instant,
    url: String,
    text: Bytes,
}

/// Provides a RESTful web server managing some Todos.
///
/// API will be:
///
/// - `GET /buses`: return a JSON list of buses.
fn main() {
    pretty_env_logger::init();

    // Turn our state into a Filter so we can combine it easily later
    let bus_cache = TimedString {
        time: Instant::now() - Duration::from_secs(30),
        url: String::from("http://developer.itsmarta.com/BRDRestService/RestBusRealTimeService/GetAllBus"),
        text: Bytes::from(&b"<unused>"[..])
    };
    let bus_cache = Arc::new(RwLock::new(bus_cache));
    let bus_cache = warp::any().map(move || bus_cache.clone());

    let train_route = format!("http://developer.itsmarta.com/RealtimeTrain/RestServiceNextTrain/GetRealtimeArrivals?apikey={}", env::var("MARTA_TRAIN_API_KEY").unwrap());
    let trains_cache = TimedString {
        time: Instant::now() - Duration::from_secs(30),
        url: train_route,
        text: Bytes::from(&b"<unused>"[..])
    };
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

    // Start up the server...
    warp::serve(routes).run(([0, 0, 0, 0], 3030));
}


// read from cache if it's still valid, otherwise hit the URL + write
fn cache_visit(cache: Store) -> impl warp::Reply {
    {
        // base case: cache still valid, only need a read lock.
        // should be very fast
        let timed_string = cache.read().unwrap();
        if Instant::now() < timed_string.time {
            return Response::new(timed_string.text.clone());
        }
    }

    // acquire write lock
    let mut timed_string = cache.write().unwrap();
    // check if someone else acquired the write lock while we were blocked
    // and updated the cache already, so just return result
    if Instant::now() < timed_string.time {
        return Response::new(timed_string.text.clone());
    }

    // otherwise we have to update the cache, slowest case.
    timed_string.time = Instant::now() + Duration::from_secs(10);
    let resp_text = reqwest::get(&timed_string.url).unwrap().text().unwrap();
    timed_string.text = Bytes::from(resp_text);
    Response::new(timed_string.text.clone())
}
