import pandas as pd
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity

class TradingRAG:
    def __init__(self):
        self.knowledge_base = self.load_trading_knowledge()
        self.vectorizer = TfidfVectorizer(stop_words='english')
        self.doc_vectors = self.vectorizer.fit_transform(self.knowledge_base['content'])
    
    def load_trading_knowledge(self):
        """Load free trading education content"""
        # You can populate this with free trading resources
        knowledge = [
            {"topic": "RSI", "content": "RSI above 70 indicates overbought conditions, below 30 indicates oversold..."},
            {"topic": "Support Resistance", "content": "Support levels act as price floors where buying interest emerges..."},
            {"topic": "Risk Management", "content": "Never risk more than 2% of portfolio on single trade..."},
            # Add more free trading knowledge
        ]
        return pd.DataFrame(knowledge)
    
    def retrieve_relevant_info(self, query, top_k=3):
        """Find relevant trading knowledge for user query"""
        query_vector = self.vectorizer.transform([query])
        similarities = cosine_similarity(query_vector, self.doc_vectors).flatten()
        top_indices = similarities.argsort()[-top_k:][::-1]
        
        relevant_docs = []
        for idx in top_indices:
            if similarities[idx] > 0.1:  # Minimum similarity threshold
                relevant_docs.append({
                    'topic': self.knowledge_base.iloc[idx]['topic'],
                    'content': self.knowledge_base.iloc[idx]['content'],
                    'similarity': similarities[idx]
                })
        return relevant_docs
