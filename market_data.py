import yfinance as yf
import pandas as pd
import streamlit as st

def get_crypto_data(symbol="BTC-USD", period="1mo"):
    """Get free crypto data from Yahoo Finance"""
    try:
        ticker = yf.Ticker(symbol)
        data = ticker.history(period=period)
        return data
    except Exception as e:
        st.error(f"Error getting data: {e}")
        return None

def calculate_simple_signals(data):
    """Calculate basic trading signals"""
    # Simple Moving Averages (free indicators)
    data['SMA_20'] = data['Close'].rolling(window=20).mean()
    data['SMA_50'] = data['Close'].rolling(window=50).mean()
    
    # RSI (free indicator)
    delta = data['Close'].diff()
    gain = (delta.where(delta > 0, 0)).rolling(window=14).mean()
    loss = (-delta.where(delta < 0, 0)).rolling(window=14).mean()
    rs = gain / loss
    data['RSI'] = 100 - (100 / (1 + rs))
    
    # Simple Buy/Sell signals
    data['Signal'] = 'HOLD'
    data.loc[(data['SMA_20'] > data['SMA_50']) & (data['RSI'] < 70), 'Signal'] = 'BUY'
    data.loc[(data['SMA_20'] < data['SMA_50']) & (data['RSI'] > 30), 'Signal'] = 'SELL'
    
    return data