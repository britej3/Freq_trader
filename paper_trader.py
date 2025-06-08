import pandas as pd
import streamlit as st
from datetime import datetime
import json

class PaperTrader:
    def __init__(self, initial_balance=10000):
        self.balance = initial_balance
        self.positions = {}
        self.trade_history = []
        self.initial_balance = initial_balance
    
    def buy(self, symbol, amount, price):
        """Execute paper buy order"""
        cost = amount * price
        if cost <= self.balance:
            self.balance -= cost
            if symbol in self.positions:
                # Average down
                total_amount = self.positions[symbol]['amount'] + amount
                avg_price = ((self.positions[symbol]['amount'] * self.positions[symbol]['price']) + cost) / total_amount
                self.positions[symbol] = {'amount': total_amount, 'price': avg_price}
            else:
                self.positions[symbol] = {'amount': amount, 'price': price}
            
            self.trade_history.append({
                'timestamp': datetime.now(),
                'action': 'BUY',
                'symbol': symbol,
                'amount': amount,
                'price': price,
                'balance': self.balance
            })
            return True
        return False
    
    def sell(self, symbol, amount, price):
        """Execute paper sell order"""
        if symbol in self.positions and self.positions[symbol]['amount'] >= amount:
            self.balance += amount * price
            self.positions[symbol]['amount'] -= amount
            
            if self.positions[symbol]['amount'] == 0:
                del self.positions[symbol]
            
            self.trade_history.append({
                'timestamp': datetime.now(),
                'action': 'SELL',
                'symbol': symbol,
                'amount': amount,
                'price': price,
                'balance': self.balance
            })
            return True
        return False
    
    def get_portfolio_value(self, current_prices):
        """Calculate total portfolio value"""
        total_value = self.balance
        for symbol, position in self.positions.items():
            if symbol in current_prices:
                total_value += position['amount'] * current_prices[symbol]
        return total_value
    
    def get_profit_loss(self, current_prices):
        """Calculate profit/loss"""
        current_value = self.get_portfolio_value(current_prices)
        return current_value - self.initial_balance