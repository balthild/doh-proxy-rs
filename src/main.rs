#![feature(async_await, await_macro, futures_api)]

mod args;
mod server;

fn main() {
    let args = &*args::ARGS;

    if args.server {
        server::run(&args.listen);
    } else if args.client {
        // TODO
    } else {
        println!("Must specify either --server or --client");
    }
}
