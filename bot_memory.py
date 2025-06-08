import json
from datetime import datetime

class BotMemory:
    def __init__(self):
        self.conversation_history = []
        self.trading_patterns = {}
        self.user_preferences = {}
    
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
