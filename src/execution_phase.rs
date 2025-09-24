use anyhow::Result;
use seda_sdk_rs::{elog, http_fetch, log, Process};
use serde::{Deserialize, Serialize};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

#[derive(Serialize, Deserialize)]
struct PolyMarketEvent {
    closed: bool,
    markets: Vec<PolyMarketMarket>,
}

#[derive(Serialize, Deserialize)]
struct PolyMarketMarket {
    #[serde(rename = "outcomePrices")]
    outcome_prices: String, // PolyMarket returns this as a JSON string, not an array
}


#[derive(Serialize, Deserialize)]
struct Response {
    prices: Vec<f64>,
    market_status: String,
}

// ============================================================================
// EXECUTION PHASE - FETCHES LIVE DATA FROM POLYMARKET ONLY
// ============================================================================

/**
 * Executes the data request phase within the SEDA network.
 * This phase fetches event data from PolyMarket for a specific event.
 * Takes a single input: event_id
 * Loops through all markets in the event and extracts the first outcome price from each market.
 * Returns a JSON response with a vector of first outcome prices and whether the event is closed.
 */
pub fn execution_phase() -> Result<()> {
    // Retrieve the input parameters for the data request (DR).
    // Expected to be a single event_id
    let dr_inputs_raw = String::from_utf8(Process::get_inputs())?;
    let event_id = dr_inputs_raw.trim();

    log!("Fetching event data from PolyMarket for event: {}", event_id);

    // Fetch PolyMarket event data
    let polymarket_event_response = http_fetch(
        format!("https://gamma-api.polymarket.com/events/{}", event_id),
        None,
    );

    // Check if the event request was successful
    if !polymarket_event_response.is_ok() {
        elog!(
            "PolyMarket HTTP Response was rejected: {} - {}",
            polymarket_event_response.status,
            String::from_utf8(polymarket_event_response.bytes)?
        );
        Process::error("Error while fetching PolyMarket event information".as_bytes());
        return Ok(());
    }
    
    // Parse event information
    let poly_market_event_data = serde_json::from_slice::<PolyMarketEvent>(&polymarket_event_response.bytes)?;
    
    // Validate that the event has at least one market
    if poly_market_event_data.markets.is_empty() {
        elog!("Event has no markets available");
        Process::error("Error: Event has no markets".as_bytes());
        return Ok(());
    }
    
    // Loop through all markets and extract the first outcome price from each
    let mut yes_prices: Vec<f64> = Vec::new();
    
    for (i, market) in poly_market_event_data.markets.iter().enumerate() {
        // Parse the outcome_prices JSON string into a vector
        let outcome_prices_array: Vec<String> = serde_json::from_str(&market.outcome_prices)?;
        
        if outcome_prices_array.is_empty() {
            elog!("Market {} has no outcome prices", i);
            continue;
        }
        
        // Get the first outcome price and parse it as f64
        let yes_price = outcome_prices_array[0].parse::<f64>().map_err(|e| {
            elog!("Failed to parse first outcome price for market {}: {}", i, e);
            anyhow::anyhow!("Invalid price format")
        })?;
        
        yes_prices.push(yes_price);
        log!("Market {}: First outcome price = {}", i, yes_price);
    }

    log!(
        "Collected {} first outcome prices from all markets",
        yes_prices.len()
    );

    let market_status = if poly_market_event_data.closed {
        "closed".to_string()
    } else {
        "open".to_string()
    };


    // Return the PolyMarket first outcome prices and event status as JSON
    Process::success(&serde_json::to_vec(&Response {
        prices: yes_prices,
        market_status: market_status,
    })?);
    Ok(())
}
