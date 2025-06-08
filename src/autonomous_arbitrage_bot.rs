// ~/marketmaker-tools/autonomous_arbitrage_bot.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tokio::time::{sleep, Duration};
use tokio::sync::RwLock;
use std::fs::OpenOptions;
use std::io::Write;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
struct BotConfig {
    api_key: String,
    secret_key: String,
    base_url: String,
    testnet: bool,
    account_balance: f64,
    max_position_percent: f64,  // Max % of account per trade
    min_profit_usdt: f64,       // Minimum profit in USDT
    min_profit_percent: f64,    // Minimum profit percentage
    scan_interval_ms: u64,      // Milliseconds between scans
    max_daily_trades: u32,      // Safety limit on daily trades
    stop_loss_percent: f64,     // Emergency stop loss
    emergency_stop: bool,
}

#[derive(Debug, Clone)]
struct PriceData {
    symbol: String,
    bid: f64,
    ask: f64,
    timestamp: u64,
}

#[derive(Debug, Clone)]
struct ArbitrageOpportunity {
    id: String,
    path: Vec<String>,
    profit_percentage: f64,
    profit_usdt: f64,
    trade_amount: f64,
    execution_steps: Vec<TradeStep>,
    estimated_fees: f64,
    net_profit: f64,
    confidence_score: f64,
    risk_level: RiskLevel,
}

#[derive(Debug, Clone)]
enum RiskLevel {
    Low,      // High liquidity, stable pairs
    Medium,   // Medium liquidity
    High,     // Lower liquidity, higher volatility
}

#[derive(Debug, Clone)]
struct TradeStep {
    symbol: String,
    side: String,      // BUY or SELL
    quantity: f64,
    expected_price: f64,
    order_type: String,
}

#[derive(Debug, Deserialize)]
struct OrderResponse {
    #[serde(rename = "orderId")]
    order_id: u64,
    symbol: String,
    status: String,
    #[serde(rename = "executedQty")]
    executed_qty: String,
    #[serde(rename = "cummulativeQuoteQty")]
    cumulative_quote_qty: String,
    fills: Vec<Fill>,
}

#[derive(Debug, Deserialize)]
struct Fill {
    price: String,
    qty: String,
    commission: String,
    #[serde(rename = "commissionAsset")]
    commission_asset: String,
}

#[derive(Debug, Deserialize)]
struct AccountInfo {
    balances: Vec<Balance>,
}

#[derive(Debug, Deserialize)]
struct Balance {
    asset: String,
    free: String,
    locked: String,
}

#[derive(Debug, Clone)]
struct TradeResult {
    success: bool,
    opportunity_id: String,
    profit_usdt: f64,
    fees_paid: f64,
    execution_time_ms: u128,
    orders: Vec<u64>, // Order IDs
    error_message: Option<String>,
}

#[derive(Debug, Clone)]
struct BotStats {
    total_scans: u64,
    opportunities_found: u64,
    trades_executed: u64,
    successful_trades: u64,
    total_profit: f64,
    total_fees: f64,
    daily_trades: u32,
    last_reset: chrono::DateTime<chrono::Utc>,
    current_balance: f64,
    max_drawdown: f64,
    win_rate: f64,
}

struct AutonomousArbitrageBot {
    config: Arc<RwLock<BotConfig>>,
    client: Client,
    stats: Arc<RwLock<BotStats>>,
    price_cache: Arc<RwLock<HashMap<String, PriceData>>>,
    trade_history: Arc<RwLock<Vec<TradeResult>>>,
    running: Arc<RwLock<bool>>,
}

impl Clone for AutonomousArbitrageBot {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            client: self.client.clone(),
            stats: Arc::clone(&self.stats),
            price_cache: Arc::clone(&self.price_cache),
            trade_history: Arc::clone(&self.trade_history),
            running: Arc::clone(&self.running),
        }
    }
}

impl AutonomousArbitrageBot {
    fn new(config: BotConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
            
        let stats = BotStats {
            total_scans: 0,
            opportunities_found: 0,
            trades_executed: 0,
            successful_trades: 0,
            total_profit: 0.0,
            total_fees: 0.0,
            daily_trades: 0,
            last_reset: Utc::now(),
            current_balance: config.account_balance,
            max_drawdown: 0.0,
            win_rate: 0.0,
        };
        
        Self {
            config: Arc::new(RwLock::new(config)),
            client,
            stats: Arc::new(RwLock::new(stats)),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            trade_history: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    async fn start_autonomous_trading(&self) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut running = self.running.write().await;
            *running = true;
        }
        
        println!("🤖 AUTONOMOUS ARBITRAGE BOT STARTING");
        println!("{}", "=".repeat(60));
        
        let config = self.config.read().await;
        println!("💰 Account Balance: ${:.2} USDT", config.account_balance);
        println!("📊 Max Position Size: {:.1}% (${:.2})", 
                 config.max_position_percent * 100.0,
                 config.account_balance * config.max_position_percent);
        println!("🎯 Min Profit: ${:.2} USDT ({:.2}%)", 
                 config.min_profit_usdt, config.min_profit_percent);
        println!("⏱️ Scan Interval: {}ms", config.scan_interval_ms);
        println!("🛡️ Daily Trade Limit: {}", config.max_daily_trades);
        
        if config.testnet {
            println!("🧪 TESTNET MODE - Safe testing environment");
        } else {
            println!("💸 LIVE TRADING - Real money at risk!");
        }
        drop(config);
        
        // Verify API connection
        self.verify_connection().await?;
        
        // Start monitoring tasks
        let bot_clone = self.clone();
        let stats_task = tokio::spawn(async move {
            bot_clone.stats_monitor().await;
        });
        
        let bot_clone = self.clone();
        let daily_reset_task = tokio::spawn(async move {
            bot_clone.daily_reset_monitor().await;
        });
        
        let bot_clone = self.clone();
        let balance_monitor_task = tokio::spawn(async move {
            bot_clone.balance_monitor().await;
        });
        
        // Main trading loop
        self.main_trading_loop().await?;
        
        // Cleanup
        stats_task.abort();
        daily_reset_task.abort();
        balance_monitor_task.abort();
        
        Ok(())
    }
    
    async fn main_trading_loop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut consecutive_errors = 0;
        let max_consecutive_errors = 10;
        
        while *self.running.read().await {
            let start_time = Instant::now();
            
            // Check if we should continue trading
            if !self.should_continue_trading().await {
                self.log_message("⏸️ Pausing trading - limits reached or emergency stop").await;
                sleep(Duration::from_secs(60)).await;
                continue;
            }
            
            match self.execute_trading_cycle().await {
                Ok(_) => {
                    consecutive_errors = 0;
                    
                    // Dynamic scan interval based on market conditions
                    let scan_interval = self.calculate_dynamic_interval().await;
                    
                    let elapsed = start_time.elapsed();
                    if elapsed < scan_interval {
                        sleep(scan_interval - elapsed).await;
                    }
                },
                Err(e) => {
                    consecutive_errors += 1;
                    self.log_message(&format!("❌ Trading cycle error: {}", e)).await;
                    
                    if consecutive_errors >= max_consecutive_errors {
                        self.log_message("🚨 Too many consecutive errors - stopping bot").await;
                        self.emergency_stop().await;
                        break;
                    }
                    
                    // Exponential backoff on errors
                    let backoff_seconds = std::cmp::min(60, 2_u64.pow(consecutive_errors as u32));
                    sleep(Duration::from_secs(backoff_seconds)).await;
                }
            }
        }
        
        Ok(())
    }
    
    async fn execute_trading_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Update scan counter
        {
            let mut stats = self.stats.write().await;
            stats.total_scans += 1;
        }
        
        // Fetch fresh market data
        let prices = self.fetch_all_prices().await?;
        {
            let mut cache = self.price_cache.write().await;
            *cache = prices;
        }
        
        // Scan for opportunities
        let opportunities = self.scan_arbitrage_opportunities().await?;
        
        if !opportunities.is_empty() {
            {
                let mut stats = self.stats.write().await;
                stats.opportunities_found += opportunities.len() as u64;
            }
            
            self.log_message(&format!("🎯 Found {} opportunities", opportunities.len())).await;
            
            // Execute best opportunity
            if let Some(best_opportunity) = opportunities.first() {
                if self.should_execute_trade(best_opportunity).await {
                    self.execute_arbitrage_trade(best_opportunity).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn fetch_all_prices(&self) -> Result<HashMap<String, PriceData>, Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let url = format!("{}/api/v3/ticker/bookTicker", config.base_url);
        drop(config);
        
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;
        
        let mut prices = HashMap::new();
        
        if let Some(tickers) = data.as_array() {
            for ticker in tickers {
                if let (Some(symbol), Some(bid_price), Some(ask_price)) = (
                    ticker["symbol"].as_str(),
                    ticker["bidPrice"].as_str().and_then(|s| s.parse::<f64>().ok()),
                    ticker["askPrice"].as_str().and_then(|s| s.parse::<f64>().ok()),
                ) {
                    prices.insert(symbol.to_string(), PriceData {
                        symbol: symbol.to_string(),
                        bid: bid_price,
                        ask: ask_price,
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    });
                }
            }
        }
        
        Ok(prices)
    }
    
    async fn scan_arbitrage_opportunities(&self) -> Result<Vec<ArbitrageOpportunity>, Box<dyn std::error::Error>> {
        let price_cache = self.price_cache.read().await;
        let config = self.config.read().await;
        
        let trading_triangles = self.get_optimized_triangles();
        let mut opportunities = Vec::new();
        
        // Test multiple position sizes
        let test_amounts = vec![
            config.account_balance * 0.05,  // 5% - very safe
            config.account_balance * 0.1,   // 10% - safe
            config.account_balance * 0.15,  // 15% - moderate
            config.account_balance * 0.2,   // 20% - aggressive
        ];
        
        for (pair1, pair2, pair3) in trading_triangles {
            for &amount in &test_amounts {
                if let Some(opportunity) = self.calculate_triangular_arbitrage(
                    &price_cache, &pair1, &pair2, &pair3, amount
                ).await {
                    if opportunity.net_profit >= config.min_profit_usdt &&
                       opportunity.profit_percentage >= config.min_profit_percent {
                        opportunities.push(opportunity);
                    }
                }
            }
        }
        
        // Sort by risk-adjusted profit
        opportunities.sort_by(|a, b| {
            let score_a = a.net_profit * a.confidence_score;
            let score_b = b.net_profit * b.confidence_score;
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        Ok(opportunities)
    }
    
    fn get_optimized_triangles(&self) -> Vec<(String, String, String)> {
        // Focus on high-liquidity, low-spread pairs for $400 account
        vec![
            // BTC triangles (highest liquidity)
            ("BTCUSDT".to_string(), "ETHBTC".to_string(), "ETHUSDT".to_string()),
            ("BTCUSDT".to_string(), "BNBBTC".to_string(), "BNBUSDT".to_string()),
            ("BTCUSDT".to_string(), "ADABTC".to_string(), "ADAUSDT".to_string()),
            ("BTCUSDT".to_string(), "DOGEBTC".to_string(), "DOGEUSDT".to_string()),
            ("BTCUSDT".to_string(), "LTCBTC".to_string(), "LTCUSDT".to_string()),
            ("BTCUSDT".to_string(), "DOTBTC".to_string(), "DOTUSDT".to_string()),
            
            // ETH triangles (second highest liquidity)
            ("ETHUSDT".to_string(), "BNBETH".to_string(), "BNBUSDT".to_string()),
            ("ETHUSDT".to_string(), "ADAETH".to_string(), "ADAUSDT".to_string()),
            ("ETHUSDT".to_string(), "LINKETH".to_string(), "LINKUSDT".to_string()),
            ("ETHUSDT".to_string(), "MATICETH".to_string(), "MATICUSDT".to_string()),
            
            // BNB triangles (fee discounts)
            ("BNBUSDT".to_string(), "ADABNB".to_string(), "ADAUSDT".to_string()),
            ("BNBUSDT".to_string(), "DOGEBNB".to_string(), "DOGEUSDT".to_string()),
            ("BNBUSDT".to_string(), "LTCBNB".to_string(), "LTCUSDT".to_string()),
            
            // Cross-stablecoin arbitrage (often profitable)
            ("BTCUSDT".to_string(), "BTCBUSD".to_string(), "BUSDUSDT".to_string()),
            ("ETHUSDT".to_string(), "ETHBUSD".to_string(), "BUSDUSDT".to_string()),
            ("BNBUSDT".to_string(), "BNBBUSD".to_string(), "BUSDUSDT".to_string()),
        ]
    }
    
    async fn calculate_triangular_arbitrage(
        &self,
        prices: &HashMap<String, PriceData>,
        pair1: &str,
        pair2: &str,
        pair3: &str,
        amount: f64,
    ) -> Option<ArbitrageOpportunity> {
        let price1 = prices.get(pair1)?;
        let price2 = prices.get(pair2)?;
        let price3 = prices.get(pair3)?;
        
        // Calculate both directions and return the better one
        let forward = self.calculate_direction(prices, pair1, pair2, pair3, amount, true).await;
        let reverse = self.calculate_direction(prices, pair1, pair2, pair3, amount, false).await;
        
        match (forward, reverse) {
            (Some(f), Some(r)) => if f.net_profit > r.net_profit { Some(f) } else { Some(r) },
            (Some(opp), None) | (None, Some(opp)) => Some(opp),
            _ => None,
        }
    }
    
    async fn calculate_direction(
        &self,
        prices: &HashMap<String, PriceData>,
        pair1: &str,
        pair2: &str,
        pair3: &str,
        amount: f64,
        forward: bool,
    ) -> Option<ArbitrageOpportunity> {
        let price1 = prices.get(pair1)?;
        let price2 = prices.get(pair2)?;
        let price3 = prices.get(pair3)?;
        
        let (final_amount, steps, path) = if forward {
            let btc_amount = amount / price1.ask;
            let eth_amount = btc_amount / price2.ask;
            let final_usdt = eth_amount * price3.bid;
            
            (final_usdt, vec![
                TradeStep {
                    symbol: pair1.to_string(),
                    side: "BUY".to_string(),
                    quantity: amount,
                    expected_price: price1.ask,
                    order_type: "MARKET".to_string(),
                },
                TradeStep {
                    symbol: pair2.to_string(),
                    side: "BUY".to_string(),
                    quantity: btc_amount,
                    expected_price: price2.ask,
                    order_type: "MARKET".to_string(),
                },
                TradeStep {
                    symbol: pair3.to_string(),
                    side: "SELL".to_string(),
                    quantity: eth_amount,
                    expected_price: price3.bid,
                    order_type: "MARKET".to_string(),
                },
            ], vec![pair1.to_string(), pair2.to_string(), pair3.to_string()])
        } else {
            let eth_amount = amount / price3.ask;
            let btc_amount = eth_amount * price2.bid;
            let final_usdt = btc_amount * price1.bid;
            
            (final_usdt, vec![
                TradeStep {
                    symbol: pair3.to_string(),
                    side: "BUY".to_string(),
                    quantity: amount,
                    expected_price: price3.ask,
                    order_type: "MARKET".to_string(),
                },
                TradeStep {
                    symbol: pair2.to_string(),
                    side: "SELL".to_string(),
                    quantity: eth_amount,
                    expected_price: price2.bid,
                    order_type: "MARKET".to_string(),
                },
                TradeStep {
                    symbol: pair1.to_string(),
                    side: "SELL".to_string(),
                    quantity: btc_amount,
                    expected_price: price1.bid,
                    order_type: "MARKET".to_string(),
                },
            ], vec![pair3.to_string(), pair2.to_string(), pair1.to_string()])
        };
        
        let profit_usdt = final_amount - amount;
        let profit_percentage = (profit_usdt / amount) * 100.0;
        
        // Calculate fees (0.075% with BNB, 0.1% without)
        let has_bnb = path.iter().any(|p| p.contains("BNB"));
        let fee_rate = if has_bnb { 0.075 } else { 0.1 };
        let estimated_fees = amount * (fee_rate / 100.0) * 3.0;
        let net_profit = profit_usdt - estimated_fees;
        
        // Calculate confidence score and risk level
        let (confidence_score, risk_level) = self.assess_opportunity_risk(&path, amount);
        
        Some(ArbitrageOpportunity {
            id: format!("{}-{}-{}-{}", 
                       path.join("-"), 
                       amount as u32,
                       if forward { "FWD" } else { "REV" },
                       chrono::Utc::now().timestamp()),
            path,
            profit_percentage,
            profit_usdt,
            trade_amount: amount,
            execution_steps: steps,
            estimated_fees,
            net_profit,
            confidence_score,
            risk_level,
        })
    }
    
    fn assess_opportunity_risk(&self, path: &[String], amount: f64) -> (f64, RiskLevel) {
        let mut confidence_score = 1.0;
        let mut risk_level = RiskLevel::Low;
        
        // Adjust confidence based on pairs
        for pair in path {
            if pair.contains("BTC") || pair.contains("ETH") || pair.contains("BNB") {
                confidence_score *= 1.0; // High liquidity pairs
            } else if pair.contains("ADA") || pair.contains("DOT") || pair.contains("LINK") {
                confidence_score *= 0.9; // Medium liquidity
            } else {
                confidence_score *= 0.7; // Lower liquidity
                risk_level = RiskLevel::Medium;
            }
        }
        
        // Adjust for trade size
        if amount > 100.0 {
            confidence_score *= 0.8; // Larger trades have higher slippage risk
            risk_level = RiskLevel::High;
        } else if amount > 50.0 {
            confidence_score *= 0.9;
            risk_level = RiskLevel::Medium;
        }
        
        (confidence_score, risk_level)
    }
    
    async fn should_execute_trade(&self, opportunity: &ArbitrageOpportunity) -> bool {
        let config = self.config.read().await;
        let stats = self.stats.read().await;
        
        // Check daily limits
        if stats.daily_trades >= config.max_daily_trades {
            return false;
        }
        
        // Check emergency stop
        if config.emergency_stop {
            return false;
        }
        
        // Check position size limits
        if opportunity.trade_amount > config.account_balance * config.max_position_percent {
            return false;
        }
        
        // Risk-based execution decisions
        match opportunity.risk_level {
            RiskLevel::Low => opportunity.net_profit >= config.min_profit_usdt,
            RiskLevel::Medium => opportunity.net_profit >= config.min_profit_usdt * 1.5,
            RiskLevel::High => opportunity.net_profit >= config.min_profit_usdt * 2.0,
        }
    }
    
    async fn execute_arbitrage_trade(&self, opportunity: &ArbitrageOpportunity) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        self.log_message(&format!("🚀 Executing trade: {}", opportunity.id)).await;
        self.log_message(&format!("   Path: {}", opportunity.path.join(" → "))).await;
        self.log_message(&format!("   Expected profit: ${:.4}", opportunity.net_profit)).await;
        
        let mut trade_result = TradeResult {
            success: false,
            opportunity_id: opportunity.id.clone(),
            profit_usdt: 0.0,
            fees_paid: 0.0,
            execution_time_ms: 0,
            orders: Vec::new(),
            error_message: None,
        };
        
        // Check balance before execution
        if !self.verify_sufficient_balance(opportunity.trade_amount).await? {
            trade_result.error_message = Some("Insufficient balance".to_string());
            self.record_trade_result(trade_result).await;
            return Ok(());
        }
        
        // Execute trades sequentially
        let mut current_amount = opportunity.trade_amount;
        let mut total_fees = 0.0;
        
        for (i, step) in opportunity.execution_steps.iter().enumerate() {
            match self.execute_market_order(&step.symbol, &step.side, current_amount).await {
                Ok(order) => {
                    trade_result.orders.push(order.order_id);
                    
                    // Calculate fees and update amount for next step
                    let step_fees: f64 = order.fills.iter()
                        .map(|fill| fill.commission.parse::<f64>().unwrap_or(0.0))
                        .sum();
                    total_fees += step_fees;
                    
                    // Update amount for next trade
                    if step.side == "SELL" {
                        current_amount = order.executed_qty.parse::<f64>().unwrap_or(0.0);
                    } else {
                        current_amount = order.fills.iter()
                            .map(|fill| fill.qty.parse::<f64>().unwrap_or(0.0))
                            .sum();
                    }
                    
                    self.log_message(&format!("   ✅ Step {}: {} completed", i + 1, step.symbol)).await;
                    
                    // Brief pause between orders
                    sleep(Duration::from_millis(100)).await;
                },
                Err(e) => {
                    trade_result.error_message = Some(format!("Step {} failed: {}", i + 1, e));
                    if i > 0 {
                        self.log_message("🚨 PARTIAL EXECUTION - Manual intervention may be needed").await;
                    }
                    break;
                }
            }
        }
        
        // Calculate final results
        trade_result.execution_time_ms = start_time.elapsed().as_millis();
        trade_result.fees_paid = total_fees;
        
        if trade_result.error_message.is_none() {
            trade_result.success = true;
            trade_result.profit_usdt = current_amount - opportunity.trade_amount;
            
            self.log_message(&format!("✅ Trade completed successfully")).await;
            self.log_message(&format!("   Profit: ${:.6} USDT", trade_result.profit_usdt)).await;
            self.log_message(&format!("   Fees: ${:.6} USDT", trade_result.fees_paid)).await;
            self.log_message(&format!("   Net: ${:.6} USDT", trade_result.profit_usdt - trade_result.fees_paid)).await;
        } else {
            self.log_message(&format!("❌ Trade failed: {:?}", trade_result.error_message)).await;
        }
        
        self.record_trade_result(trade_result).await;
        Ok(())
    }
    
    // Helper methods for API calls, monitoring, etc.
    fn generate_signature(&self, query_string: &str, secret_key: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query_string.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
    
    async fn execute_market_order(
        &self,
        symbol: &str,
        side: &str,
        quantity: f64,
    ) -> Result<OrderResponse, Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        
        let quantity_str = format!("{:.8}", quantity);
        let mut query_params = vec![
            ("symbol", symbol),
            ("side", side),
            ("type", "MARKET"),
            ("timestamp", &timestamp.to_string()),
        ];
        
        if side == "BUY" {
            query_params.push(("quoteOrderQty", &quantity_str));
        } else {
            query_params.push(("quantity", &quantity_str));
        }
        
        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
            
        let signature = self.generate_signature(&query_string, &config.secret_key);
        let final_query = format!("{}&signature={}", query_string, signature);
        
        let url = format!("{}/api/v3/order", config.base_url);
        
        let response = self.client
            .post(&url)
            .header("X-MBX-APIKEY", &config.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(final_query)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Order failed: {}", error_text).into());
        }
        
        let order: OrderResponse = response.json().await?;
        Ok(order)
    }
    
    async fn verify_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let url = format!("{}/api/v3/time", config.base_url);
        let _response: Value = self.client.

// Test authenticated endpoint
        let account_info = self.get_account_info().await?;
        let usdt_balance = account_info.balances.iter()
            .find(|b| b.asset == "USDT")
            .map(|b| b.free.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
            
        println!("✅ API Connection verified");
        println!("💰 Current USDT Balance: ${:.2}", usdt_balance);
        
        // Update config with actual balance
        let mut config_write = self.config.write().await;
        config_write.account_balance = usdt_balance;
        drop(config_write);
        
        Ok(())
    }
    
    async fn get_account_info(&self) -> Result<AccountInfo, Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let query_string = format!("timestamp={}", timestamp);
        let signature = self.generate_signature(&query_string, &config.secret_key);
        
        let url = format!("{}/api/v3/account?{}&signature={}", 
                         config.base_url, query_string, signature);
        
        let response = self.client
            .get(&url)
            .header("X-MBX-APIKEY", &config.api_key)
            .send()
            .await?;
            
        let account: AccountInfo = response.json().await?;
        Ok(account)
    }
    
    async fn verify_sufficient_balance(&self, required_amount: f64) -> Result<bool, Box<dyn std::error::Error>> {
        let account = self.get_account_info().await?;
        let usdt_balance = account.balances.iter()
            .find(|b| b.asset == "USDT")
            .map(|b| b.free.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
            
        Ok(usdt_balance >= required_amount)
    }
    
    async fn record_trade_result(&self, result: TradeResult) {
        {
            let mut stats = self.stats.write().await;
            stats.trades_executed += 1;
            stats.daily_trades += 1;
            
            if result.success {
                stats.successful_trades += 1;
                stats.total_profit += result.profit_usdt - result.fees_paid;
                stats.current_balance += result.profit_usdt - result.fees_paid;
            }
            
            stats.total_fees += result.fees_paid;
            stats.win_rate = (stats.successful_trades as f64 / stats.trades_executed as f64) * 100.0;
            
            // Track max drawdown
            let config = self.config.read().await;
            let drawdown = ((config.account_balance - stats.current_balance) / config.account_balance) * 100.0;
            if drawdown > stats.max_drawdown {
                stats.max_drawdown = drawdown;
            }
            drop(config);
        }
        
        {
            let mut history = self.trade_history.write().await;
            history.push(result);
            
            // Keep only last 1000 trades
            if history.len() > 1000 {
                history.drain(0..100);
            }
        }
    }
    
    async fn should_continue_trading(&self) -> bool {
        let config = self.config.read().await;
        let stats = self.stats.read().await;
        
        // Check emergency stop
        if config.emergency_stop {
            return false;
        }
        
        // Check daily limits
        if stats.daily_trades >= config.max_daily_trades {
            return false;
        }
        
        // Check stop loss
        let drawdown = ((config.account_balance - stats.current_balance) / config.account_balance) * 100.0;
        if drawdown >= config.stop_loss_percent {
            self.log_message(&format!("🚨 Stop loss triggered at {:.2}% drawdown", drawdown)).await;
            return false;
        }
        
        // Check minimum balance
        if stats.current_balance < 10.0 {
            self.log_message("🚨 Balance too low to continue trading").await;
            return false;
        }
        
        true
    }
    
    async fn calculate_dynamic_interval(&self) -> Duration {
        let stats = self.stats.read().await;
        let config = self.config.read().await;
        
        let base_interval = config.scan_interval_ms;
        let mut multiplier = 1.0;
        
        // Slow down if we're making too many trades
        if stats.daily_trades > config.max_daily_trades / 2 {
            multiplier *= 2.0;
        }
        
        // Speed up if we haven't found opportunities recently
        if stats.opportunities_found == 0 && stats.total_scans > 100 {
            multiplier *= 0.5;
        }
        
        Duration::from_millis((base_interval as f64 * multiplier) as u64)
    }
    
    async fn emergency_stop(&self) {
        {
            let mut config = self.config.write().await;
            config.emergency_stop = true;
        }
        
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        
        self.log_message("🚨 EMERGENCY STOP ACTIVATED").await;
        
        // Cancel all open orders (if any)
        if let Err(e) = self.cancel_all_orders().await {
            self.log_message(&format!("Warning: Failed to cancel orders: {}", e)).await;
        }
    }
    
    async fn cancel_all_orders(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let query_string = format!("timestamp={}", timestamp);
        let signature = self.generate_signature(&query_string, &config.secret_key);
        
        let url = format!("{}/api/v3/openOrders?{}&signature={}", 
                         config.base_url, query_string, signature);
        
        let _response = self.client
            .delete(&url)
            .header("X-MBX-APIKEY", &config.api_key)
            .send()
            .await?;
            
        Ok(())
    }
    
    async fn stats_monitor(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
        
        while *self.running.read().await {
            interval.tick().await;
            
            let stats = self.stats.read().await;
            let config = self.config.read().await;
            
            println!("\n📊 BOT PERFORMANCE UPDATE");
            println!("{}", "=".repeat(40));
            println!("🔍 Total Scans: {}", stats.total_scans);
            println!("🎯 Opportunities Found: {}", stats.opportunities_found);
            println!("📈 Trades Executed: {} / {} daily", stats.daily_trades, config.max_daily_trades);
            println!("✅ Success Rate: {:.1}%", stats.win_rate);
            println!("💰 Total Profit: ${:.4} USDT", stats.total_profit);
            println!("💸 Total Fees: ${:.4} USDT", stats.total_fees);
            println!("📊 Current Balance: ${:.2} USDT", stats.current_balance);
            println!("📉 Max Drawdown: {:.2}%", stats.max_drawdown);
            
            let profit_rate = if stats.total_scans > 0 {
                (stats.opportunities_found as f64 / stats.total_scans as f64) * 100.0
            } else { 0.0 };
            println!("🎲 Opportunity Rate: {:.3}%", profit_rate);
            
            // Alert if performance is concerning
            if stats.win_rate < 60.0 && stats.trades_executed > 10 {
                println!("⚠️ LOW WIN RATE ALERT");
            }
            
            if stats.max_drawdown > 5.0 {
                println!("⚠️ HIGH DRAWDOWN ALERT");
            }
            
            println!("{}", "=".repeat(40));
        }
    }
    
    async fn daily_reset_monitor(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
        
        while *self.running.read().await {
            interval.tick().await;
            
            let should_reset = {
                let stats = self.stats.read().await;
                let now = Utc::now();
                now.signed_duration_since(stats.last_reset).num_hours() >= 24
            };
            
            if should_reset {
                let mut stats = self.stats.write().await;
                stats.daily_trades = 0;
                stats.last_reset = Utc::now();
                
                self.log_message("🔄 Daily limits reset").await;
            }
        }
    }
    
    async fn balance_monitor(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1800)); // 30 minutes
        
        while *self.running.read().await {
            interval.tick().await;
            
            if let Ok(account) = self.get_account_info().await {
                let usdt_balance = account.balances.iter()
                    .find(|b| b.asset == "USDT")
                    .map(|b| b.free.parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(0.0);
                
                {
                    let mut stats = self.stats.write().await;
                    if (stats.current_balance - usdt_balance).abs() > 0.01 {
                        stats.current_balance = usdt_balance;
                        self.log_message(&format!("💰 Balance updated: ${:.2} USDT", usdt_balance)).await;
                    }
                }
                
                // Update config balance
                {
                    let mut config = self.config.write().await;
                    config.account_balance = usdt_balance;
                }
            }
        }
    }
    
    async fn log_message(&self, message: &str) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let log_entry = format!("[{}] {}", timestamp, message);
        
        println!("{}", log_entry);
        
        // Write to log file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("arbitrage_bot.log") {
            let _ = writeln!(file, "{}", log_entry);
        }
    }
    
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        
        self.log_message("🛑 Bot stopped by user").await;
    }
    
    pub async fn get_status(&self) -> String {
        let stats = self.stats.read().await;
        let config = self.config.read().await;
        
        format!(
            "Status: {} | Balance: ${:.2} | Trades: {}/{} | Win Rate: {:.1}% | Profit: ${:.4}",
            if *self.running.read().await { "RUNNING" } else { "STOPPED" },
            stats.current_balance,
            stats.daily_trades,
            config.max_daily_trades,
            stats.win_rate,
            stats.total_profit
        )
    }
}

// Main execution function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 BINANCE AUTONOMOUS ARBITRAGE BOT v2.0");
    println!("Optimized for $400 USDT accounts with advanced risk management");
    println!("{}", "=".repeat(80));
    
    // Configuration - UPDATE THESE WITH YOUR API CREDENTIALS
    let config = BotConfig {
        // Method 1: Environment variables (recommended)
        api_key: std::env::var("BINANCE_API_KEY")
            .unwrap_or_else(|_| "YOUR_API_KEY_HERE".to_string()),
        secret_key: std::env::var("BINANCE_SECRET_KEY")
            .unwrap_or_else(|_| "YOUR_SECRET_KEY_HERE".to_string()),
        
        // Method 2: Direct replacement (less secure)
        // api_key: "YOUR_API_KEY_HERE".to_string(),
        // secret_key: "YOUR_SECRET_KEY_HERE".to_string(),
        base_url: "https://testnet.binance.vision".to_string(), // Change to https://api.binance.com for live
        testnet: true, // Set to false for live trading
        account_balance: 400.0, // Will be updated with actual balance
        max_position_percent: 0.15, // Max 15% per trade for $400 account
        min_profit_usdt: 0.25, // Minimum $0.25 profit
        min_profit_percent: 0.1, // Minimum 0.1% profit
        scan_interval_ms: 2000, // Scan every 2 seconds
        max_daily_trades: 50, // Conservative daily limit
        stop_loss_percent: 10.0, // 10% account stop loss
        emergency_stop: false,
    };
    
    let bot = AutonomousArbitrageBot::new(config);
    
    // Setup graceful shutdown
    let bot_clone = bot.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        println!("\n🛑 Shutdown signal received...");
        bot_clone.stop().await;
    });
    
    // Start the bot
    match bot.start_autonomous_trading().await {
        Ok(_) => println!("✅ Bot shutdown completed"),
        Err(e) => println!("❌ Bot error: {}", e),
    }
    
    Ok(())
}
        
