/// Streaming support for real-time data quality validation.
///
/// Enables validation of streaming data with window-based aggregation:
/// - Tumbling windows: Fixed-size, non-overlapping windows
/// - Sliding windows: Fixed-size, overlapping windows
/// - Session windows: Event-driven windows based on gaps in data
///
/// Example DSL:
/// ```text
/// dataset events {
///     schema { timestamp: datetime, value: float }
///     stream {
///         window: tumbling(60s)
///         watermark: 30s
///     }
/// }
/// ```

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub enum WindowType {
    Tumbling(Duration),
    Sliding(Duration, Duration), // (size, slide)
    Session(Duration),
}

#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub window: WindowType,
    pub watermark: Option<Duration>,
    pub allowed_lateness: Option<Duration>,
}

#[derive(Debug)]
pub struct StreamingWindow {
    pub window_id: String,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub events: Vec<(SystemTime, String)>, // (timestamp, event)
    pub is_closed: bool,
}

impl StreamingWindow {
    pub fn new(window_id: String, start: SystemTime, end: SystemTime) -> Self {
        Self {
            window_id,
            start_time: start,
            end_time: end,
            events: Vec::new(),
            is_closed: false,
        }
    }

    pub fn add_event(&mut self, timestamp: SystemTime, event: String) -> Result<(), String> {
        if timestamp < self.start_time || timestamp > self.end_time {
            return Err(format!("Event timestamp outside window bounds"));
        }
        self.events.push((timestamp, event));
        Ok(())
    }

    pub fn close(&mut self) {
        self.is_closed = true;
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

pub struct TumblingWindowExecutor {
    window_size: Duration,
    current_windows: HashMap<String, StreamingWindow>,
}

impl TumblingWindowExecutor {
    pub fn new(window_size: Duration) -> Self {
        Self {
            window_size,
            current_windows: HashMap::new(),
        }
    }

    pub fn get_window_for_event(
        &mut self,
        timestamp: SystemTime,
    ) -> Result<&mut StreamingWindow, String> {
        let duration_since_epoch = timestamp
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?;

        let window_number = duration_since_epoch.as_secs() / self.window_size.as_secs();
        let window_id = format!("window_{}", window_number);

        if !self.current_windows.contains_key(&window_id) {
            let window_start = UNIX_EPOCH + Duration::from_secs(window_number * self.window_size.as_secs());
            let window_end = window_start + self.window_size;
            let window = StreamingWindow::new(window_id.clone(), window_start, window_end);
            self.current_windows.insert(window_id.clone(), window);
        }

        Ok(self.current_windows.get_mut(&window_id).unwrap())
    }

    pub fn close_windows_before(&mut self, cutoff_time: SystemTime) -> Vec<StreamingWindow> {
        let mut closed = Vec::new();

        self.current_windows.retain(|_, window| {
            if window.end_time < cutoff_time {
                window.close();
                closed.push(window.clone());
                false
            } else {
                true
            }
        });

        closed
    }

    pub fn get_closed_windows(&self) -> Vec<&StreamingWindow> {
        self.current_windows
            .values()
            .filter(|w| w.is_closed)
            .collect()
    }
}

pub struct SlidingWindowExecutor {
    window_size: Duration,
    slide_interval: Duration,
    current_windows: HashMap<String, StreamingWindow>,
}

impl SlidingWindowExecutor {
    pub fn new(window_size: Duration, slide_interval: Duration) -> Self {
        Self {
            window_size,
            slide_interval,
            current_windows: HashMap::new(),
        }
    }

    pub fn get_windows_for_event(
        &mut self,
        timestamp: SystemTime,
    ) -> Result<Vec<&mut StreamingWindow>, String> {
        let duration_since_epoch = timestamp
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?;

        let mut windows = Vec::new();
        let mut slide_count = 0;

        loop {
            let slide_start = slide_count * self.slide_interval.as_secs();
            let window_start = UNIX_EPOCH + Duration::from_secs(slide_start);
            let window_end = window_start + self.window_size;

            if timestamp >= window_start && timestamp <= window_end {
                let window_id = format!("sliding_{}_{}", slide_count, slide_start);

                if !self.current_windows.contains_key(&window_id) {
                    let window = StreamingWindow::new(window_id.clone(), window_start, window_end);
                    self.current_windows.insert(window_id.clone(), window);
                }

                windows.push(self.current_windows.get_mut(&window_id).unwrap());
            }

            if window_end > timestamp {
                break;
            }

            slide_count += 1;
        }

        Ok(windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tumbling_window_creation() {
        let window_size = Duration::from_secs(60);
        let executor = TumblingWindowExecutor::new(window_size);
        assert_eq!(executor.current_windows.len(), 0);
    }

    #[test]
    fn test_window_add_event() {
        let start = UNIX_EPOCH + Duration::from_secs(0);
        let end = UNIX_EPOCH + Duration::from_secs(60);
        let mut window = StreamingWindow::new("test".to_string(), start, end);

        let event_time = UNIX_EPOCH + Duration::from_secs(30);
        assert!(window.add_event(event_time, "event1".to_string()).is_ok());
        assert_eq!(window.event_count(), 1);
    }

    #[test]
    fn test_window_reject_out_of_bounds() {
        let start = UNIX_EPOCH + Duration::from_secs(0);
        let end = UNIX_EPOCH + Duration::from_secs(60);
        let mut window = StreamingWindow::new("test".to_string(), start, end);

        let out_of_bounds = UNIX_EPOCH + Duration::from_secs(90);
        assert!(window.add_event(out_of_bounds, "event".to_string()).is_err());
    }
}
