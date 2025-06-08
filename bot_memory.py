import json
import streamlit as st
from datetime import datetime

class BotMemory:
    def __init__(self):
        self.conversation_history = []
        self.trading_patterns = {}
        self.user_preferences = {}
        self.trade_outcomes = []
    
    def add_conversation(self, user_input, ai_response, market_context):
        """Store conversation with market context"""
        memory_entry = {
            'timestamp': datetime.now().isoformat(),
            'user_input': user_input,
            'ai_response': ai_response,
            'market_context': market_context,
            'portfolio_state': self.get_portfolio_snapshot()
        }
        self.conversation_history.append(memory_entry)
    
    def get_relevant_context(self, current_query, limit=5):
        """Retrieve relevant past conversations"""
        # Simple keyword matching (can be enhanced)
        relevant = []
        for entry in self.conversation_history[-20:]:
            if any(word in entry['user_input'].lower()
                   for word in current_query.lower().split()):
                relevant.append(entry)
        return relevant[-limit:]

    def get_portfolio_snapshot(self):
        """Return a basic snapshot of the current portfolio."""
        trader = st.session_state.get('trader')
        if trader is None:
            return {}
        return {
            'balance': trader.balance,
            'positions': trader.positions.copy()
        }

    def record_trade(self, action, symbol, amount, price, result):
        """Store outcome of a paper trade for simple learning"""
        self.trade_outcomes.append({
            'timestamp': datetime.now().isoformat(),
            'action': action,
            'symbol': symbol,
            'amount': amount,
            'price': price,
            'result': result
        })

    def summarize_performance(self):
        """Return basic stats about past trades."""
        if not self.trade_outcomes:
            return {}
        wins = sum(1 for t in self.trade_outcomes if t['result'] > 0)
        losses = len(self.trade_outcomes) - wins
        avg_pl = sum(t['result'] for t in self.trade_outcomes) / len(self.trade_outcomes)
        return {
            'total_trades': len(self.trade_outcomes),
            'wins': wins,
            'losses': losses,
            'avg_pl': avg_pl
        }
