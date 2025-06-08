import streamlit as st
import google.generativeai as genai
import plotly.graph_objects as go
from market_data import get_crypto_data, calculate_simple_signals

# Configure AI
genai.configure(api_key="YOUR_API_KEY_HERE")
model = genai.GenerativeModel('gemini-1.5-flash')

st.title("🚀 Smart Trading Bot - Free Version")

# Sidebar for settings
st.sidebar.header("Settings")
crypto = st.sidebar.selectbox("Choose Crypto", ["BTC-USD", "ETH-USD", "ADA-USD"])
period = st.sidebar.selectbox("Time Period", ["1mo", "3mo", "6mo", "1y"])

# Get and display market data
if st.sidebar.button("Get Market Data"):
    with st.spinner("Fetching free market data..."):
        data = get_crypto_data(crypto, period)
        if data is not None:
            data_with_signals = calculate_simple_signals(data)
            
            # Create chart
            fig = go.Figure()
            fig.add_trace(go.Candlestick(x=data.index,
                                       open=data['Open'],
                                       high=data['High'],
                                       low=data['Low'],
                                       close=data['Close'],
                                       name="Price"))
            fig.add_trace(go.Scatter(x=data.index, y=data_with_signals['SMA_20'], 
                                   name="SMA 20", line=dict(color='blue')))
            fig.add_trace(go.Scatter(x=data.index, y=data_with_signals['SMA_50'], 
                                   name="SMA 50", line=dict(color='red')))
            
            st.plotly_chart(fig, use_container_width=True)
            
            # Show latest signal
            latest_signal = data_with_signals['Signal'].iloc[-1]
            latest_price = data_with_signals['Close'].iloc[-1]
            
            st.metric("Current Price", f"${latest_price:.2f}")
            st.metric("Signal", latest_signal)

# Enhanced chat with market context
if "messages" not in st.session_state:
    st.session_state.messages = []

for message in st.session_state.messages:
    with st.chat_message(message["role"]):
        st.markdown(message["content"])

if prompt := st.chat_input("Ask about the market..."):
    st.session_state.messages.append({"role": "user", "content": prompt})
    with st.chat_message("user"):
        st.markdown(prompt)
    
    with st.chat_message("assistant"):
        # Get current market data for context
        current_data = get_crypto_data(crypto, "1mo")
        if current_data is not None:
            signals_data = calculate_simple_signals(current_data)
            latest_info = f"""
            Current {crypto} price: ${signals_data['Close'].iloc[-1]:.2f}
            RSI: {signals_data['RSI'].iloc[-1]:.1f}
            Signal: {signals_data['Signal'].iloc[-1]}
            """
            
            enhanced_prompt = f"""
            Market Context: {latest_info}
            User Question: {prompt}
            
Please provide trading advice based on current market data.
            """
        else:
            enhanced_prompt = prompt
        
        response = model.generate_content(enhanced_prompt)
        st.markdown(response.text)
        st.session_state.messages.append({"role": "assistant", "content": response.text})
