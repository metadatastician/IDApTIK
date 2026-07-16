//! The grounded-network explorer mode: shows the current vantage and reachable
//! nodes, and lets you resolve/ping/traceroute/ssh/hack/scrub/exit.
mod app;
mod render;
pub use app::run;
