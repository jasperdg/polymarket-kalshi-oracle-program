use anyhow::Result;
use seda_sdk_rs::{elog, get_reveals, log, Process};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
struct Response {
    prices: Vec<f64>,
    market_status: String,
}

/**
 * Executes the tally phase within the SEDA network.
 * This phase aggregates the results (e.g., price data) revealed during the execution phase,
 * calculates the median value for each index position across all reveals, and submits the final result.
 * Note: The number of reveals depends on the replication factor set in the data request parameters.
 */
pub fn tally_phase() -> Result<()> {
    // Tally inputs can be retrieved from Process.getInputs(), though it is unused in this example.
    // let tally_inputs = Process::get_inputs();

    // Retrieve consensus reveals from the tally phase.
    let reveals = get_reveals()?;
    let mut market_status: Option<String> = None;
    let mut all_price_vectors: Vec<Vec<f64>> = Vec::new();

    // Iterate over each reveal and collect price vectors
    for reveal in reveals {
        let response = serde_json::from_slice::<Response>(&reveal.body.reveal)?;
        
        // Check market_status consensus
        if market_status.is_none() {
            market_status = Some(response.market_status);
        } else if market_status.as_ref().unwrap() != &response.market_status {
            elog!("Market status is inconsistent between reveals");
            Process::error("Market status is inconsistent between reveals".as_bytes());
            return Ok(());
        }
        
        // Collect price vectors for median calculation
        log!("Received prices: {:?}", response.prices);
        all_price_vectors.push(response.prices);
    }

    if all_price_vectors.is_empty() {
        // If no valid prices were revealed, report an error indicating no consensus.
        Process::error("No consensus among revealed results".as_bytes());
        return Ok(());
    }

    // Find the maximum vector length to determine how many indices we need to process
    let max_length = all_price_vectors.iter().map(|v| v.len()).max().unwrap_or(0);
    
    if max_length == 0 {
        elog!("All price vectors are empty");
        Process::error("All price vectors are empty".as_bytes());
        return Ok(());
    }

    // Calculate median for each index position
    let mut median_prices: Vec<f64> = Vec::new();
    
    for index in 0..max_length {
        let mut values_at_index: Vec<f64> = Vec::new();
        
        // Collect all values at this index from all reveals
        for price_vector in &all_price_vectors {
            if index < price_vector.len() {
                values_at_index.push(price_vector[index]);
            }
        }
        
        if values_at_index.is_empty() {
            log!("No values found at index {}, skipping", index);
            continue;
        }
        
        let median_value = median(values_at_index);
        median_prices.push(median_value);
        log!("Index {}: Median = {}", index, median_value);
    }

    log!("Final median prices: {:?}", median_prices);

    // Return the median result
    Process::success(&serde_json::to_vec(&Response {
        prices: median_prices,
        market_status: market_status.unwrap(),
    })?);

    Ok(())
}

fn median(mut nums: Vec<f64>) -> f64 {
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let middle = nums.len() / 2;

    if nums.len() % 2 == 0 {
        return (nums[middle - 1] + nums[middle]) / 2.0;
    }

    nums[middle]
}