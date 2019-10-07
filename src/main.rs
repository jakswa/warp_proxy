#![deny(warnings)]
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
extern crate serde_derive;
extern crate warp;
extern crate reqwest;
extern crate bytes;
use std::time::{Duration, Instant};
//use std::env;

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
/// - `GET /busses`: return a JSON list of busses.
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

    let busses = warp::path("busses");
    let busses_index = busses.and(warp::path::end());

    // `GET /busses`
    let list = warp::get2()
        .and(busses_index)
        .and(bus_cache.clone())
        .map(cache_visit);

    // View access logs by setting `RUST_LOG=busses`.
    let routes = list.with(warp::log("busses"));
    //    .or(list2.with(warp::log("trains")));

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
