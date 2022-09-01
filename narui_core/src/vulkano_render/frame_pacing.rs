use derivative::Derivative;
use std::{
    collections::VecDeque,
    time::{Duration, Instant, SystemTime},
};

#[derive(Default)]
pub(crate) struct FramePacer {
    base_offset: i64,
    refresh: RunningMean<25>,
    processing_time: RunningMean<25>,

    render_loop_begin_to_present_time: RunningMean<25>,
    swapchain_present_to_present_time: RunningMean<25>,

    render_loop_begin_to_swapchain_present_time: RunningMean<25>,

    render_start_to_present_delay: RunningMean<25>,
    base_seq: i64,

    render_loop_start: i64,                // time in nsecs
    render_loop_start_hist: VecDeque<i64>, // time in nsecs

    swapchain_present_time: VecDeque<i64>, // time in nsecs

    last_frame_seq: i64,
    render_start: Option<i64>,
    refresh_factor: i64,
    clockid: libc::clockid_t,
}

// we try to estimate the time we need for our renderloop by measuring the time
// elapsed between render_loop_begin and render_loop_end
// we try to estimate the present time by measuring the time between
// render_loop_begin and present_time for that frame
// furthermore we try to figure out the vsync beam by looking at present_time
// and its sequence number finally we sleep in render_loop_end by an amount that
// delays the next processing of events and r

impl FramePacer {
    pub fn new(clockid: libc::clockid_t) -> Self {
        FramePacer { last_frame_seq: -1, refresh_factor: 1, clockid, ..Default::default() }
    }

    fn get_time(&self) -> i64 {
        let mut timespec = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        unsafe { libc::clock_gettime(self.clockid, &mut timespec as *mut _) };
        return (timespec.tv_sec as i64) * 1_000_000_000 + timespec.tv_nsec as i64;
    }

    pub fn want_redraw(&mut self) -> Option<Instant> {
        // we can only do frame pacing after the first frame
        if self.refresh.inited() {
            let now_inst = Instant::now();
            let now = self.get_time();

            // try to figure out the time to the next timeslot for a frame to be presented
            let next_frame_seq = ((now - self.base_offset) as f64 / self.refresh.get()) as i64 + 1;
            let next_frame_seq = if next_frame_seq % self.refresh_factor == 0 {
                next_frame_seq
            } else {
                next_frame_seq - next_frame_seq % self.refresh_factor + self.refresh_factor
            };
            dbg!(next_frame_seq, self.last_frame_seq);
            let next_frame_seq = if next_frame_seq == self.last_frame_seq {
                //                println!("two frames for same target, delaying target");
                next_frame_seq + self.refresh_factor
            } else {
                next_frame_seq
            };


            let next_frame_time = self.frame_time_for_seq(next_frame_seq);
            let diff = next_frame_time - now;

            let refresh = self.refresh.get();
            let fudge = (refresh / 8.0) as i64;

            // give 1ms extra
            // let actual_time = self.processing_time.get() as i64 - 1_000_000;
            let actual_time = self.render_loop_begin_to_swapchain_present_time.get() as i64 - fudge;
            let needed_time = actual_time + fudge + fudge;

            dbg!(
                self.refresh.get() / 1e6,
                self.refresh_factor,
                diff as f64 / 1e6,
                actual_time as f64 / 1e6,
                next_frame_seq,
                self.processing_time.get() / 1e6,
                self.render_loop_begin_to_present_time.get() / 1e6,
                self.render_loop_begin_to_swapchain_present_time.get() / 1e6,
                self.swapchain_present_to_present_time.get() / 1e6
            );

            // we wont manage to render to display in this frame
            if diff < actual_time {
                if dbg!(diff + self.get_refresh() as i64) < needed_time {
                    None
                } else {
                    println!("wont make this target, sleeping for the next");
                    // TODO(robin): we probably only ever want to sleep to the next frame, not
                    // further
                    Some(now_inst + Duration::from_nanos(diff as u64) / 15)
                }
            } else if diff < needed_time {
                // render
                self.last_frame_seq = next_frame_seq;
                println!("rendering for seq {}", next_frame_seq);
                None
            } else {
                // we are too early, sleep a bit
                println!("too early, sleeping a bit");
                Some(now_inst + Duration::from_nanos(((diff - needed_time) / 10) as u64))
            }
        } else {
            None
        }
    }

    fn get_refresh(&self) -> f64 { self.refresh.get() * self.refresh_factor as f64 }

    fn frame_time_for_seq(&self, seq: i64) -> i64 {
        self.base_offset + (self.refresh.get() * seq as f64) as i64
    }

    pub fn render_loop_begin(&mut self) { self.render_loop_start = self.get_time(); }

    pub fn render_loop_end(&mut self) {
        let time = self.get_time();
        println!("renderloop {time}");
        self.render_loop_start_hist.push_front(self.render_loop_start);

        self.processing_time.push((time - self.render_loop_start) as f64);

        // TODO(robin): revisit when we replace processing time with better estimate
        if self.refresh.inited() {
            // TODO(robin): this should be correct, because we can overlap gpu and cpu
            // but how do we schedule in want_redraw then?
            // let processing_time = self.processing_time.get();
            //
            let processing_time = self.render_loop_begin_to_swapchain_present_time.get();

            if processing_time > (self.get_refresh() - 2_000_000.0) {
                self.refresh_factor += 1;
            } else if processing_time < (self.get_refresh() - self.refresh.get()) {
                self.refresh_factor = (self.refresh_factor - 1).max(1);
            }
        }
    }

    pub fn discarded_frame(&mut self) {
        println!("discarded a frame");
        self.render_loop_start_hist.pop_back().unwrap();
        self.swapchain_present_time.pop_back().unwrap();
    }

    pub fn present_time(&mut self, refresh: i64, time: i64, seq: i64) {
        let render_loop_begin_time = self.render_loop_start_hist.pop_back().unwrap();
        let swapchain_present_time = self.swapchain_present_time.pop_back().unwrap();

        // dbg!(&self.render_loop_start_hist);
        // dbg!(&self.swapchain_present_time);
        // dbg!(render_loop_begin_time, swapchain_present_time, time);

        self.render_loop_begin_to_present_time.push((time - render_loop_begin_time) as f64);
        self.swapchain_present_to_present_time.push((time - swapchain_present_time) as f64);

        if self.base_offset == 0 {
            self.base_offset = time;
            self.base_seq = seq;
        } else {
            dbg!(seq);
            let seq = seq - self.base_seq;
            let refresh = (time - self.base_offset) as f64 / seq as f64;
            let refresh = self.refresh.push(refresh);
            let expected = self.base_offset + (refresh * seq as f64) as i64;
            println!("got frame for {seq}")
        }
    }

    pub fn swapchain_present(&mut self) {
        let time = self.get_time();
        println!("swapchain present {time}");
        self.swapchain_present_time.push_front(time);

        dbg!(&self.swapchain_present_time, &self.render_loop_start_hist);

        let diff_time = (self.swapchain_present_time.back().unwrap()
            - self.render_loop_start_hist.back().unwrap()) as f64;
        println!("render_loop_begin_to_swapchain_present_time {}", diff_time / 1e6);
        self.render_loop_begin_to_swapchain_present_time.push(diff_time);
    }
}

#[derive(Derivative)]
#[derivative(Default(new = "true"))]
struct RunningMean<const N: usize> {
    #[derivative(Default(value = "[0.0; N]"))]
    values: [f64; N],
    idx: usize,
    mean: f64,
    n: usize,
}

impl<const N: usize> RunningMean<N> {
    fn get(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.mean / self.n as f64
        }
    }

    fn inited(&self) -> bool { self.n != 0 }

    fn push(&mut self, value: f64) -> f64 {
        let old = self.values[self.idx];
        self.values[self.idx] = value;
        self.idx = (self.idx + 1) % N;
        self.mean = self.mean + value - old;
        self.n = (self.n + 1).min(N);

        self.mean / self.n as f64
    }
}
