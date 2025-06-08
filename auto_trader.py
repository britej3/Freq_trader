import time
import streamlit as st
from market_data import get_crypto_data, calculate_simple_signals
from paper_trader import PaperTrader
import google.generativeai as genai

class AutoTrader:
    def __init__(self, trader, model):
        self.trader = trader
        self.model = model
        self.is_running = False
        self.trade_log = []
    
    def analyze_and_trade(self, symbol):
        """Get AI analysis and execute trades if conditions met"""
        try:
            # Get fresh market data
            data = get_crypto_data(symbol, "1mo")
            if data is None:
                return "No data available"
            
            signals_data = calculate_simple_signals(data)
            current_price = signals_data['Close'].iloc[-1]
            current_signal = signals_data['Signal'].iloc[-1]
            rsi = signals_data['RSI'].iloc[-1]
            
            # AI Analysis
            analysis_prompt = f"""
            Analyze this trading situation:
            - Symbol: {symbol}
            - Current Price: ${current_price:.2f}
            - Technical Signal: {current_signal}
            - RSI: {rsi:.1f}
            - Available Cash: ${self.trader.balance:.2f}
            - Current Position: {self.trader.positions.get(symbol, 'None')}
            
            Should I BUY, SELL, or HOLD? Give a one-word answer followed by reasoning.
            Consider risk management and position sizing.
            """
            
            response = self.model.generate_content(analysis_prompt)
            ai_decision = response.text.strip()
            
            # Execute based on AI decision
            action_taken = "NONE"
            if "BUY" in ai_decision.upper() and self.trader.balance > current_price * 0.01:
                # Buy 1% of balance worth
                amount = (self.trader.balance * 0.01) / current_price
                if self.trader.buy(symbol, amount, current_price):
                    action_taken = f"BOUGHT {amount:.4f} {symbol}"
            
            elif "SELL" in ai_decision.upper() and symbol in self.trader.positions:
                # Sell 50% of position
                amount = self.trader.positions[symbol]['amount'] * 0.5
                if self.trader.sell(symbol, amount, current_price):
                    action_taken = f"SOLD {amount:.4f} {symbol}"
            
            log_entry = {
                'timestamp': time.strftime('%Y-%m-%d %H:%M:%S'),
                'price': current_price,
                'ai_analysis': ai_decision[:100],
                'action': action_taken,
                'balance': self.trader.balance
            }
            
            self.trade_log.append(log_entry)
            return log_entry
            
        except Exception as e:
            return f"Error: {str(e)}"