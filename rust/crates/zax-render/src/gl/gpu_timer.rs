//! The GPU time of the scene passes (Rust-only, for the dynamic resolution of the handheld):
//! timer queries read a few frames late, so that measuring never waits for the GPU.

use std::time::Duration;

use glow::Query;

use super::api::Gl;

/// Queries in flight: a result is read once the GPU has it, two or three frames after its frame.
const QUERIES: usize = 3;

/// Times each frame's scene passes with a ring of queries.
#[derive(Debug)]
pub struct GpuTimer {
    queries: [Query; QUERIES],
    /// Whether each query holds a frame whose result has not been read.
    pending: [bool; QUERIES],
    /// The query of the frame being drawn.
    next: usize,
    /// Whether a query is open (between `begin` and `end`).
    open: bool,
}

impl GpuTimer {
    /// A timer for `gl`, or `None` when the context cannot time GPU work.
    pub fn new(gl: &Gl) -> Option<Self> {
        if !gl.supports_timer_queries() {
            return None;
        }
        let mut queries = Vec::with_capacity(QUERIES);
        for _ in 0..QUERIES {
            let Ok(query) = gl.create_query() else {
                for query in queries {
                    gl.delete_query(query);
                }
                return None;
            };
            queries.push(query);
        }
        let queries: [Query; QUERIES] = queries.try_into().ok()?;
        // Some OpenGL ES drivers list the extension and refuse the query: one is tried first,
        // its errors told apart from those recorded before.
        for _ in 0..16 {
            if gl.take_error() == glow::NO_ERROR {
                break;
            }
        }
        gl.begin_time_elapsed(queries[0]);
        gl.end_time_elapsed();
        if gl.take_error() != glow::NO_ERROR {
            for query in queries {
                gl.delete_query(query);
            }
            return None;
        }
        Some(Self {
            queries,
            pending: [false; QUERIES],
            next: 0,
            open: false,
        })
    }

    /// Starts timing a frame, unless its query still waits for an earlier result.
    pub fn begin(&mut self, gl: &Gl) {
        if self.pending[self.next] {
            return;
        }
        gl.begin_time_elapsed(self.queries[self.next]);
        self.open = true;
    }

    /// Ends the frame begun last.
    pub fn end(&mut self, gl: &Gl) {
        if !std::mem::take(&mut self.open) {
            return;
        }
        gl.end_time_elapsed();
        self.pending[self.next] = true;
        self.next = (self.next + 1) % QUERIES;
    }

    /// Deletes the queries.
    pub fn delete(self, gl: &Gl) {
        for query in self.queries {
            gl.delete_query(query);
        }
    }

    /// The GPU time of the oldest frame whose result arrived, if one did: never waits.
    pub fn collect(&mut self, gl: &Gl) -> Option<Duration> {
        let mut latest = None;
        for offset in 0..QUERIES {
            let index = (self.next + offset) % QUERIES;
            if !self.pending[index] || !gl.query_ready(self.queries[index]) {
                continue;
            }
            self.pending[index] = false;
            let nanoseconds = gl.query_nanoseconds(self.queries[index]);
            latest = Some(Duration::from_nanos(u64::from(nanoseconds)));
        }
        // A disjoint period (power state change) makes the results meaningless.
        if gl.gpu_disjoint() {
            return None;
        }
        latest
    }
}
