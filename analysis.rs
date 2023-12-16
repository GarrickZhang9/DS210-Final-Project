use csv::Reader;
use std::error::Error;
use std::fs::File;
use plotters::prelude::*;
use chrono::{TimeZone, Utc};

pub fn main(generation: usize, last_transac: i64) -> Result<(), Box<dyn Error>> {
    let processed_data = read_and_process_csv("trust_scores.csv")?;
    println!("Graph generation: {}", generation);
    let formatted_time = format_epoch_to_gmt_string(last_transac);
    println!("Timeframe: 11/8/2010 18:45:12 - {}", formatted_time);
    perform_analysis(&processed_data)?;
    Ok(())
}

// process the data to become more human-readable trust scores, export it to a 2 dimension vector
fn read_and_process_csv(file_path: &str) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = Reader::from_reader(file);
    let mut processed_data = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let scores: Vec<f64> = record.iter() 
            .filter_map(|s| s.parse::<f64>().ok())
            .filter(|&s| s != f64::INFINITY && s != 0.0) // filter the inf and data with 0 score
            .map(|s| (1.0 / s) - 11.0) // convert each of the scores back to the range from -10 to 10
            .collect();
        if !scores.is_empty() {
            processed_data.push(scores);
        }
    }
    Ok(processed_data)
}

pub fn perform_analysis(processed_data: &[Vec<f64>]) -> Result<(), Box<dyn Error>> {
    let avg_threshold: f64 = 6.0; // manually set these up
    let stddev_threshold: f64 = 2.2;

    let mut analysis_results = Vec::new();
    let mut std_devs = Vec::new();
    let mut untrusted_scores = Vec::new();
    let mut high_stable_trust_scores = Vec::new();

    for scores in processed_data {
        let average = calculate_average(scores);
        let std_dev = calculate_std_dev(scores, average);
        analysis_results.push((average, std_dev));
        std_devs.push(std_dev);

        // filter the trusted/untrusted nodes based on my threashold
        if average < avg_threshold || std_dev > stddev_threshold {
            untrusted_scores.push(average);
        }
        if average >= avg_threshold && std_dev <= stddev_threshold {
            high_stable_trust_scores.push(average);
        }
    }

    // collect all the averages and make a histogram of the current generation
    let averages: Vec<f64> = analysis_results.iter().map(|(avg, _)| *avg).collect();
    create_histogram_avg(&averages, "/Users/garrickzhang/Desktop/GZ/BU/Sophmore/DS 210/Final Project/Final_Project/avg_histogram.png")?;

    // also make a scatter plot of the standard deviations of the current generation
    create_scatter_plot_stddev(std_devs.clone(), "/Users/garrickzhang/Desktop/GZ/BU/Sophmore/DS 210/Final Project/Final_Project/stddev_scatterplot.png")?;
    
    println!("Total nodes: {}", processed_data.len());

    let mean_of_averages = statrs::statistics::Statistics::mean(&averages);
    let mean_of_std_devs = statrs::statistics::Statistics::mean(&std_devs);
    
    println!("Mean score of all the nodes: {:.4}", mean_of_averages);
    println!("Mean standard deviations of all the nodes: {:.4}", mean_of_std_devs);

    let avg_untrusted = calculate_average(&untrusted_scores);
    println!("Total untrusted nodes: {}", untrusted_scores.len());
    println!("    - Average score of these nodes: {:.4}", avg_untrusted);

    let avg_high_stable = calculate_average(&high_stable_trust_scores);
    println!("Total trusted nodes: {}", high_stable_trust_scores.len());
    println!("    - Average score of these nodes: {:.4}", avg_high_stable);

    Ok(())
}

fn calculate_average(scores: &[f64]) -> f64 {
    let sum: f64 = scores.iter().sum();
    sum / scores.len() as f64
}

fn calculate_std_dev(scores: &[f64], mean: f64) -> f64 {
    let variance: f64 = scores.iter().map(|&s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
    variance.sqrt()
}

// converter from time since epoch to human readable time (in GMT)
fn format_epoch_to_gmt_string(epoch_time: i64) -> String {
    let datetime = Utc.timestamp(epoch_time, 0);
    datetime.format("%d/%m/%Y %H:%M:%S GMT").to_string()
}


// used chat-gpt to help me create the histogram and scatterplot
fn create_histogram_avg(scores: &[f64], file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    const BIN_SIZE: f64 = 1.0; 
    const RANGE_START: i32 = -10;
    const RANGE_END: i32 = 10;
    const UPPER_LIMIT_Y: usize = 1000; 
    let title = "Histogram of Average Trust Scores";
        
    let root = BitMapBackend::new(file_path, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 40))
        .x_label_area_size(50)
        .y_label_area_size(40)
        .margin(10)
        .build_cartesian_2d(RANGE_START..RANGE_END, 0..UPPER_LIMIT_Y)?;

    chart.configure_mesh().draw()?;

    // Create bins for the histogram
    let mut bins: [usize; (RANGE_END - RANGE_START) as usize] = [0; (RANGE_END - RANGE_START) as usize];
    for &score in scores {
        let bin_index = ((score - RANGE_START as f64) / BIN_SIZE) as usize;
        if bin_index < bins.len() {
            bins[bin_index] += 1;
        }
    }

    chart.configure_mesh()
        .x_labels((RANGE_END - RANGE_START) as usize) // Set the number of labels to the range size
        .x_label_formatter(&|x| format!("{}", x))
        .y_desc("Count")
        .x_desc("Average Trust Score")
        .draw()?;

    // Convert bin counts to plotters data format
    let histogram_data: Vec<(i32, usize)> = bins
        .iter()
        .enumerate()
        .map(|(index, &count)| (RANGE_START + index as i32, count))
        .collect();

    chart.draw_series(
        Histogram::vertical(&chart)
            .style(RED.mix(0.5).filled())
            .data(histogram_data),
    )?;

    root.present()?;
    Ok(())
}

fn create_scatter_plot_stddev(std_devs: Vec<f64>, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(file_path, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let data: Vec<(f64, f64)> = std_devs.into_iter().enumerate()
        .map(|(index, std_dev)| (index as f64, std_dev))
        .collect();

    let x_range = plotters::data::fitting_range(data.iter().map(|x| &x.0));
    let y_range = 0.0..15.0; // Set the y-axis range manually

    let mut chart = ChartBuilder::on(&root)
        .caption("Scatter Plot of Standard Deviations", ("sans-serif", 40))
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(10)
        .build_cartesian_2d(x_range, y_range)?;

    chart.configure_mesh()
        .x_desc("Node Label") // Label for x-axis
        .y_desc("Standard Deviation") // Label for y-axis
        .draw()?;

    chart.draw_series(
        data.iter().map(|&point| {
            Circle::new(point, 3, RED.filled())
        })
    )?;

    root.present()?;
    Ok(())
}
