import streamlit as st
import google.generativeai as genai
import time
import pandas as pd
from market_data import get_crypto_data, calculate_simple_signals
from paper_trader import PaperTrader
from auto_trader import AutoTrader
from bot_memory import BotMemory
from trading_rag import TradingRAG

# Page config
st.set_page_config(page_title="Free AI Trading Bot", layout="wide")

# Initialize
genai.configure(api_key="YOUR_API_KEY_HERE")
model = genai.GenerativeModel('gemini-1.5-flash')

if 'trader' not in st.session_state:
    st.session_state.trader = PaperTrader(10000)
if 'auto_trader' not in st.session_state:
    st.session_state.auto_trader = AutoTrader(st.session_state.trader, model)
if 'bot_memory' not in st.session_state:
    st.session_state.bot_memory = BotMemory()
if 'trading_rag' not in st.session_state:
    st.session_state.trading_rag = TradingRAG()

st.title("🤖 Complete Free AI Trading Bot")

# Three columns layout
col1, col2, col3 = st.columns([1, 2, 1])

with col1:
    st.subheader("Portfolio")
    
    # Current stats
    data = get_crypto_data("BTC-USD", "1d")
    if data is not None:
        current_price = data['Close'].iloc[-1]
        portfolio_value = st.session_state.trader.get_portfolio_value({"BTC-USD": current_price})
        profit_loss = portfolio_value - 10000
        
        st.metric("Total Value", f"${portfolio_value:.2f}")
        st.metric("Cash", f"${st.session_state.trader.balance:.2f}")
        st.metric("P&L", f"${profit_loss:.2f}", f"{(profit_loss/10000)*100:.1f}%")
    
    # Auto-trading controls
    st.subheader("Auto Trading")
    
    if st.button("🚀 Start Auto Trading"):
        result = st.session_state.auto_trader.analyze_and_trade("BTC-USD")
        st.success(f"Analysis complete: {result}")
    
    if st.button("📊 View Auto Log"):
        if st.session_state.auto_trader.trade_log:
            log_df = pd.DataFrame(st.session_state.auto_trader.trade_log)
            st.dataframe(log_df.tail(5))

    # Display simple learning stats
    mem = st.session_state.get('bot_memory')
    if mem and mem.trade_outcomes:
        stats = mem.summarize_performance()
        st.subheader("Learning Stats")
        st.write(f"Trades: {stats['total_trades']} | Wins: {stats['wins']} | Losses: {stats['losses']} | Avg P/L: {stats['avg_pl']:.2f}")

with col2:
    st.subheader("Market Analysis")
    
    # Market data and chart
    crypto = st.selectbox("Select Crypto", ["BTC-USD", "ETH-USD", "ADA-USD"])
    
    if st.button("📈 Analyze Market"):
        with st.spinner("Getting AI analysis..."):
            data = get_crypto_data(crypto, "1mo")
            if data is not None:
                signals_data = calculate_simple_signals(data)
                
                # Show chart
                import plotly.graph_objects as go
                fig = go.Figure()
                fig.add_trace(go.Scatter(x=data.index, y=data['Close'], 
                                       name="Price", line=dict(color='blue')))
                fig.add_trace(go.Scatter(x=data.index, y=signals_data['SMA_20'], 
                                       name="SMA 20", line=dict(color='orange')))
                st.plotly_chart(fig, use_container_width=True)
                
                # AI Analysis
                analysis_prompt = f"""
                Analyze {crypto} for a beginner trader:
                Current Price: ${signals_data['Close'].iloc[-1]:.2f}
                RSI: {signals_data['RSI'].iloc[-1]:.1f}
                Signal: {signals_data['Signal'].iloc[-1]}
                
                Provide simple, actionable advice in 3 sentences.
                """
                
                response = model.generate_content(analysis_prompt)
                st.info(response.text)

        
def enhanced_ai_response(user_input, current_market_data):
    """AI response with memory and RAG"""
    
    # 1. Retrieve relevant past conversations
    conversation_context = st.session_state.bot_memory.get_relevant_context(user_input)
    
    # 2. Retrieve relevant trading knowledge
    knowledge_context = st.session_state.trading_rag.retrieve_relevant_info(user_input)
    
    # 3. Build enhanced prompt
    enhanced_prompt = f"""
    TRADING KNOWLEDGE CONTEXT:
    {chr(10).join([f"- {doc['topic']}: {doc['content'][:200]}..." for doc in knowledge_context])}
    
    CONVERSATION HISTORY:
    {chr(10).join([f"Previous: {conv['user_input']} -> {conv['ai_response'][:100]}..." for conv in conversation_context])}
    
    CURRENT MARKET DATA:
    {current_market_data}
    
    CURRENT USER QUESTION: {user_input}
    
    Provide a comprehensive answer using the knowledge base and conversation history.
    Reference previous discussions when relevant.
    """
    
    response = model.generate_content(enhanced_prompt)
    
    # 4. Store this interaction in memory
    st.session_state.bot_memory.add_conversation(
        user_input, 
        response.text, 
        current_market_data
    )
    
    return response.text


with col3:
    st.subheader("AI Chat")
    
    # Simple chat interface
    if "chat_messages" not in st.session_state:
        st.session_state.chat_messages = []
    
    # Display messages
    for msg in st.session_state.chat_messages[-5:]:  # Show last 5
        st.text(f"{msg['role']}: {msg['content'][:50]}...")
    
    # Input
    user_input = st.text_input("Ask the AI:")
    if st.button("Send") and user_input:
        st.session_state.chat_messages.append({"role": "user", "content": user_input})
        
        # Get current market data
        data = get_crypto_data("BTC-USD", "1mo")
        if data is not None:
            signals_data = calculate_simple_signals(data)
            current_price = signals_data['Close'].iloc[-1]
            current_signal = signals_data['Signal'].iloc[-1]
            current_rsi = signals_data['RSI'].iloc[-1]
        
            current_market_context = f"""
            Current BTC-USD Price: ${current_price:.2f}
            Signal: {current_signal}
            RSI: {current_rsi:.1f}
            """
        
        enhanced_response = enhanced_ai_response(user_input, current_market_context)
        st.session_state.chat_messages.append({"role": "ai", "content": enhanced_response})
        st.rerun()

# Footer with instructions
st.markdown("---")
st.markdown("""
### 🎯 How to Use This Free Trading Bot:
1. **Monitor Portfolio**: Check your paper trading performance
2. **Analyze Markets**: Get AI insights on crypto trends  
3. **Auto Trade**: Let AI make trading decisions (paper money only)
4. **Chat with AI**: Ask questions about trading strategies
5. **Learn**: This is 100% free practice - no real money at risk!
""")
