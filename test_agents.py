import urllib.request
import urllib.error
import json
import threading
import time
import random

import os
API_KEY = os.environ.get("OPENAI_API_KEY", "sk-proj-YOUR_API_KEY_HERE")
BASE_URL = "http://localhost:4000/proxy/openai/v1/chat/completions"

AGENTS = [
    "research-agent-01",
    "qa-bot-alpha",
    "customer-support-agent",
    "finance-analyst-bot",
    "code-reviewer-ai",
    "marketing-copywriter",
    "data-summarizer",
    "devops-assistant",
    "security-scanner-bot",
    "hr-assistant-agent"
]

MESSAGES = [
    "What is the capital of France?",
    "Explain quantum computing in one sentence.",
    "Write a haiku about programming.",
    "What is 2 + 2?",
    "Tell me a short joke."
]

def make_request(agent_id, message, idx):
    data = json.dumps({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": message}]
    }).encode("utf-8")
    
    req = urllib.request.Request(BASE_URL, data=data)
    req.add_header("Authorization", f"Bearer {API_KEY}")
    req.add_header("Content-Type", "application/json")
    req.add_header("x-govrix-scout-agent-id", agent_id)
    
    try:
        print(f"[{agent_id}] Sending request {idx}...")
        start_time = time.time()
        with urllib.request.urlopen(req) as response:
            result = json.loads(response.read().decode())
            print(f"[{agent_id}] Received response for request {idx} in {time.time() - start_time:.2f}s")
            
    except urllib.error.URLError as e:
        body = ""
        if hasattr(e, 'read'):
            body = e.read().decode()
        print(f"[{agent_id}] Request {idx} failed: {e} - Body: {body}")

def run_agent(agent_id):
    # Each agent sends a few requests
    for i in range(3):
        msg = random.choice(MESSAGES)
        make_request(agent_id, msg, i+1)
        time.sleep(random.uniform(0.5, 2.0))

print("Starting agent testing...")
threads = []
for agent in AGENTS:
    t = threading.Thread(target=run_agent, args=(agent,))
    threads.append(t)
    t.start()

for t in threads:
    t.join()

print("All agent testing completed.")
