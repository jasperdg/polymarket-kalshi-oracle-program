use anyhow::Result;
use seda_sdk_rs::{elog, http_fetch, log, Process};
use serde::{Deserialize, Serialize};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

#[derive(Serialize, Deserialize)]
struct PolyMarketMarket {
    #[serde(rename = "outcomePrices")]
    outcome_prices: String, // PolyMarket returns this as a JSON string, not an array
    volume: String,
}

#[derive(Serialize, Deserialize)]
struct KalshiMarket {
    yes_bid_dollars: String,
    volume: u64,
}

#[derive(Serialize, Deserialize)]
struct KalshiMarketResponse {
    market: KalshiMarket,
}

// ============================================================================
// EXECUTION PHASE - FETCHES LIVE DATA FROM KALSHI & POLYMARKET - TAKES VOLUME WEIGHTED PRICE
// ============================================================================

/**
 * Executes the data request phase within the SEDA network.
 * This phase fetches price data from both Kalshi and PolyMarket for the same market,
 * then calculates a volume-weighted average price between the two platforms.
 * Currently works with binary prediction markets and focuses on the "yes" outcome price.
 * Returns the volume-weighted average price as the final result.
 */
pub fn execution_phase() -> Result<()> {
    // Retrieve the input parameters for the data request (DR).
    // Expected to be a market identifier that works for both Kalshi and PolyMarket APIs.
    let dr_inputs_raw = String::from_utf8(Process::get_inputs())?;
    let dr_inputs_trimmed = dr_inputs_raw.trim();

    let market_tickers: Vec<&str> = dr_inputs_trimmed.split(',').collect();

    log!("Fetching market data from Kalshi and PolyMarket for market: {} and {}", market_tickers[0], market_tickers[1]);

    // Step 1: Fetch Kalshi market data (yes bid price and volume)
    let kalshi_market_response = http_fetch(
                format!("https://api.elections.kalshi.com/trade-api/v2/markets/{}", market_tickers[0]),
        None,
    );


    // Check if the market request was successful
    if !kalshi_market_response.is_ok() {
        elog!(
            "market HTTP Response was rejected: {} - {}",
            kalshi_market_response.status,
            String::from_utf8(kalshi_market_response.bytes)?
        );
        Process::error("Error while fetching market information".as_bytes());
        return Ok(());
    }
    
    
    // Parse market informationmarket_response
    let kalshi_market_data = serde_json::from_slice::<KalshiMarketResponse>(&kalshi_market_response.bytes)?;
    log!(
        "Fetched Kalshi Price (YES BID): {} cents with volume {}",
        kalshi_market_data.market.yes_bid_dollars,
        kalshi_market_data.market.volume
    );

    let kalshi_yes_bid_dollars = kalshi_market_data.market.yes_bid_dollars.parse::<f64>()?;


    // Step 2: Fetch PolyMarket market data (yes outcome price and volume)
    let polymarket_market_response = http_fetch(
                format!("https://gamma-api.polymarket.com/markets/{}", market_tickers[1]),
        None,
    );


    // Check if the market request was successful
    if !polymarket_market_response.is_ok() {
        elog!(
            "market HTTP Response was rejected: {} - {}",
            polymarket_market_response.status,
            String::from_utf8(polymarket_market_response.bytes)?
        );
        Process::error("Error while fetching market information".as_bytes());
        return Ok(());
    }
    
    // Parse market informationmarket_response
    let poly_market_market_data = serde_json::from_slice::<PolyMarketMarket>(&polymarket_market_response.bytes)?;
    let outcome_prices_array: Vec<String> = serde_json::from_str(&poly_market_market_data.outcome_prices)?;
    let polymarket_yes_price = outcome_prices_array[0].parse::<f64>()?; // 0 = yes price

    log!(
        "Fetched Price (YES PRICE): {} cents with volume {}",
        polymarket_yes_price,
        poly_market_market_data.volume
    );


    let poly_market_volume = poly_market_market_data.volume.parse::<f64>()?;

    // Step 3: Calculate volume-weighted average price between Kalshi and PolyMarket
    let total_volume = kalshi_market_data.market.volume as f64 + poly_market_volume;
    
    if total_volume == 0.0 {
        elog!("Total volume is zero, cannot calculate volume weighted average price");
        Process::error("Error: Total volume is zero".as_bytes());
        return Ok(());
    }

    // Calculate weighted prices: price × volume for each platform
    let kalshi_weighted_price = kalshi_yes_bid_dollars * (kalshi_market_data.market.volume as f64);
    let polymarket_weighted_price = polymarket_yes_price * poly_market_volume;

    // Final volume-weighted average: (sum of weighted prices) / (total volume)
    let volume_weighted_average_price = (kalshi_weighted_price + polymarket_weighted_price) / total_volume;

    log!(
        "Volume Weighted Average Price: {:.8} cents",
        volume_weighted_average_price
    );

    // Return the volume-weighted average price as the execution result
    Process::success(volume_weighted_average_price.to_string().as_bytes());
    Ok(())
}
