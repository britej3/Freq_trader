import streamlit as st
import google.generativeai as genai
from market_data import get_crypto_data, calculate_simple_signals
from paper_trader import PaperTrader
import plotly.graph_objects as go

# Initialize session state
if 'trader' not in st.session_state:
    st.session_state.trader = PaperTrader(10000)

# Configure AI
genai.configure(api_key="YOUR_API_KEY_HERE")
model = genai.GenerativeModel('gemini-1.5-flash')

st.title("🎯 Complete Trading Bot - Paper Trading")

# Sidebar
st.sidebar.header("Paper Trading Dashboard")
crypto = st.sidebar.selectbox("Crypto", ["BTC-USD", "ETH-USD"])

# Get current data
data = get_crypto_data(crypto, "1mo")
if data is not None:
    signals_data = calculate_simple_signals(data)
    current_price = signals_data['Close'].iloc[-1]
    current_signal = signals_data['Signal'].iloc[-1]
    
    # Portfolio display
    portfolio_value = st.session_state.trader.get_portfolio_value({crypto: current_price})
    profit_loss = st.session_state.trader.get_profit_loss({crypto: current_price})
    
    st.sidebar.metric("Portfolio Value", f"${portfolio_value:.2f}")
    st.sidebar.metric("Cash Balance", f"${st.session_state.trader.balance:.2f}")
    st.sidebar.metric("Profit/Loss", f"${profit_loss:.2f}", 
                     delta=f"{(profit_loss/10000)*100:.1f}%")
    
    # Manual trading
    st.sidebar.subheader("Manual Trading")
    trade_amount = st.sidebar.number_input("Amount to Trade", min_value=0.001, value=0.1, step=0.001)
    
    col1, col2 = st.sidebar.columns(2)
    with col1:
        if st.button("🟢 BUY"):
            if st.session_state.trader.buy(crypto, trade_amount, current_price):
                st.success(f"Bought {trade_amount} {crypto} at ${current_price:.2f}")
            else:
                st.error("Insufficient balance!")
    
    with col2:
        if st.button("🔴 SELL"):
            if st.session_state.trader.sell(crypto, trade_amount, current_price):
                st.success(f"Sold {trade_amount} {crypto} at ${current_price:.2f}")
            else:
                st.error("Insufficient position!")

# Main chat interface with trading capabilities
st.subheader("AI Trading Assistant")

if "messages" not in st.session_state:
    st.session_state.messages = []

for message in st.session_state.messages:
    with st.chat_message(message["role"]):
        st.markdown(message["content"])

if prompt := st.chat_input("Ask for trading advice or request trades..."):
    st.session_state.messages.append({"role": "user", "content": prompt})
    with st.chat_message("user"):
        st.markdown(prompt)
    
    with st.chat_message("assistant"):
        # Enhanced context with portfolio info
        portfolio_context = f"""
        Current Portfolio:
        - Cash: ${st.session_state.trader.balance:.2f}
        - Positions: {st.session_state.trader.positions}
        - Total Value: ${portfolio_value:.2f}
        - P&L: ${profit_loss:.2f}
        
        Market Data:
        - {crypto} Price: ${current_price:.2f}
        - Signal: {current_signal}
        - RSI: {signals_data['RSI'].iloc[-1]:.1f}
        
        User Request: {prompt}
        
        Provide specific trading advice. If user wants to execute trades, 
        suggest specific amounts and reasoning.
        """
        
        response = model.generate_content(portfolio_context)
        st.markdown(response.text)
        st.session_state.messages.append({"role": "assistant", "content": response.text})

# Display trade history
if st.session_state.trader.trade_history:
    st.subheader("Trade History")
    trade_df = pd.DataFrame(st.session_state.trader.trade_history)
    st.dataframe(trade_df.tail(10))