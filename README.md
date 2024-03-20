# Rchan

4chan client written in Rust.

## Components

- Simple API client
- Streaming API client

## Usage

### API Client

The `rchan-api` crate is a simple 4chan API wrapper.
If you just want to query the 4chan API, `rchan-api` is all you need.
It has built in rate limiting and if-modified-since support.

### Streaming API Client

The `rchan-stream` crate is a streaming 4chan API wrapper.
It uses the `rchan-api` crate to periodically poll the 4chan API for new posts.
It uses an internal state to keep track of posts it has already seen, and only emits new posts.
If it cannot fetch a thread, it will retry so that no posts are missed.


