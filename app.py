import streamlit as st
import google.generativeai as genai
import os

# Configure Gemini (free tier)
genai.configure(api_key="YOUR_API_KEY_HERE")
model = genai.GenerativeModel('gemini-1.5-flash')  # Free model

st.title("🤖 My Trading Bot Assistant")

# Chat interface
if "messages" not in st.session_state:
    st.session_state.messages = []

# Display chat history
for message in st.session_state.messages:
    with st.chat_message(message["role"]):
        st.markdown(message["content"])

# User input
if prompt := st.chat_input("Ask me about trading..."):
    # Add user message
    st.session_state.messages.append({"role": "user", "content": prompt})
    with st.chat_message("user"):
        st.markdown(prompt)
    
    # Get AI response
    with st.chat_message("assistant"):
        response = model.generate_content(f"Trading question: {prompt}")
        st.markdown(response.text)
        st.session_state.messages.append({"role": "assistant", "content": response.text})
