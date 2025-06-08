# Freq_trader

This repository demonstrates a free, Gemini-powered trading bot built with Python and Streamlit.  
It uses Yahoo Finance for market data and provides paper trading to practice strategies without risking real funds.

## Features

- Chat interface backed by Google's Gemini (free tier)
- Live crypto data from Yahoo Finance
- Technical indicators: moving averages and RSI
- Paper trading portfolio with buy/sell history
- Optional auto-trading logic
- Conversation memory and a small retrieval-augmented knowledge base

## Setup

1. **Get a free Gemini API key** from [ai.google.dev](https://ai.google.dev) and note the rate limits.
2. **Create a Python environment**:
   ```bash
   python -m venv bot-env
   source bot-env/bin/activate  # on Windows use bot-env\Scripts\activate
   ```
3. **Install dependencies**:
   ```bash
   pip install streamlit pandas requests python-binance google-generativeai
   pip install plotly ccxt ta-lib-binary yfinance scikit-learn pickle5
   ```
4. **Add your API key** to `final_app.py` where indicated.
5. **Run the app**:
   ```bash
   streamlit run final_app.py
   ```
The interface will open in your browser at `http://localhost:8501`.

## Docker

1. Build the image:
   ```bash
   docker build -t freq_trader .
   ```
2. Run the container:
   ```bash
   docker run -p 8501:8501 freq_trader
   ```
   Then open `http://localhost:8501` in your browser.

## Notes

- All trading is simulated (paper trading). No real money is used.
- The code is provided for learning purposes. Always test strategies before using real funds.

