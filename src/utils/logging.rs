use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug)]
pub struct Results {
    title: String,                           // Title of the run TODO.
    url: String,                             // URL that was targeted.
    //socket_path: Option<String>, // UNIX Domain Socket or Windows Named Pipe that was targeted. TODO
    requests_per_second: PerformanceStats, // Number of requests that were sent per second.
    requests: u32,               // Number of requests that were sent.
    latency: PerformanceStats,          // Response latency.
    throughput: PerformanceStats,       // Response data throughput per second.(response bytes per second)
    duration: u32,               // Amount of time the test took, in seconds.
    errors: u32,                 // Number of connection errors (including timeouts) that occurred.
    timeouts: u32,               // Number of connection timeouts that occurred.
    //mismatches: u32,             // Number of requests with a mismatched body. TODO
    start: SystemTime,           // When the test started UNIX.
    finish: SystemTime,          // When the test ended UNIX.
    connections: u32,            // Amount of connections used.
    //pipelining: u32,             // Number of pipelined requests used per connection.TODO
    non2xx: u32,                 // Number of non-2xx response status codes received.
    //resets: u32, // How many times the requests pipeline was reset due to setupRequest returning a falsey value.TODO see pipeline.
    //status_code_stats: HashMap<String, u16>, // Requests counter per status code.
}

impl Results {
    pub fn new(
        title: String,
        url: String,
        requests_per_second: PerformanceStats,
        requests: u32,
        latency: PerformanceStats,
        throughput: PerformanceStats,
        duration: u32,
        errors: u32,
        timeouts: u32,
        start: SystemTime,
        finish: SystemTime,
        connections: u32,
        non2xx: u32,
    ) -> Self {
        Results {
            title,
            url,
            requests_per_second,
            requests,
            latency,
            throughput,
            duration,
            errors,
            timeouts,
            start,
            finish,
            connections,
            non2xx,
        }
    }
}

#[derive(Debug)]
pub struct PerformanceStats {
    min: f64,     // The lowest value for this statistic.
    max: f64,     // The highest value for this statistic.
    average: f64, // The average (mean) value.
    stddev: f64,  // The standard deviation.
    p2_5: f64,    // The 2.5th percentile value for this statistic.
    p50: f64,     // The 50th percentile value for this statistic.
    p75: f64,     // The 75th percentile value for this statistic.
    p90: f64,     // The 90th percentile value for this statistic.
    p97_5: f64,   // The 97.5th percentile value for this statistic.
    p99: f64,     // The 99th percentile value for this statistic.
    p99_9: f64,   // The 99.9th percentile value for this statistic.
    p99_99: f64,  // The 99.99th percentile value for this statistic.
    p99_999: f64, // The 99.999th percentile value for this statistic.
}

impl PerformanceStats {
    pub fn from_data(mut data: Vec<f64>) -> Self {
        if data.is_empty() {
            panic!("Dataset cannot be empty");
        }

        // Sort the data in ascending order.
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = data.len();

        // Compute the mean (average).
        let sum: f64 = data.iter().sum();
        let mean = sum / n as f64;

        // Compute the variance and then the standard deviation.
        let variance: f64 = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let stddev = variance.sqrt();

        // Compute percentiles using interpolation.
        let p2_5 = Self::compute_percentile(&data, 2.5);
        let p50 = Self::compute_percentile(&data, 50.0);
        let p75 = Self::compute_percentile(&data, 75.0);
        let p90 = Self::compute_percentile(&data, 90.0);
        let p97_5 = Self::compute_percentile(&data, 97.5);
        let p99 = Self::compute_percentile(&data, 99.0);
        let p99_9 = Self::compute_percentile(&data, 99.9);
        let p99_99 = Self::compute_percentile(&data, 99.99);
        let p99_999 = Self::compute_percentile(&data, 99.999);

        PerformanceStats {
            min: data[0],
            max: data[n - 1],
            average: mean,
            stddev,
            p2_5,
            p50,
            p75,
            p90,
            p97_5,
            p99,
            p99_9,
            p99_99,
            p99_999,
        }
    }

    /// Computes the given percentile using linear interpolation between points.
    fn compute_percentile(sorted_data: &[f64], percentile: f64) -> f64 {
        let n = sorted_data.len();
        if n == 1 {
            return sorted_data[0];
        }
        // Compute the rank as a fractional index.
        let rank = (percentile / 100.0) * (n as f64 - 1.0);
        let lower_index = rank.floor() as usize;
        let upper_index = rank.ceil() as usize;
        if lower_index == upper_index {
            return sorted_data[lower_index];
        }
        let weight = rank - lower_index as f64;
        sorted_data[lower_index] * (1.0 - weight) + sorted_data[upper_index] * weight
    }
}
