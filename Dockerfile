# Use official Python image
FROM python:3.11-slim

# Install system dependencies (for some packages)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set work directory
WORKDIR /app

# Copy dependency list
COPY requirements.txt ./

# Install Python dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY . .

# Expose Streamlit port
EXPOSE 8501

# Run the app
CMD ["streamlit", "run", "final_app.py", "--server.port", "8501", "--server.address", "0.0.0.0"]
