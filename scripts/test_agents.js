const apiKey = process.env.API_KEY || 'sk-proj-YOUR_API_KEY_HERE';

const agents = [
    'DataAnalyst-Agent',
    'CustomerSupport-Bot',
    'CodeReviewer-AI',
    'Research-Assistant',
    'ContentWriter-Agent',
    'SEO-Optimizer',
    'Sales-Outreach-Bot',
    'HR-Onboarding-Agent',
    'Finance-Forecaster',
    'Security-Scanner'
];

async function runTest() {
    console.log('🚀 Starting Agent Traffic Simulation...');

    for (let i = 0; i < 3; i++) {
        for (const agent of agents) {
            try {
                const payload = {
                    model: 'gpt-3.5-turbo',
                    messages: [{ role: 'user', content: `Hello! Give me a fun fact about science. Request ${i + 1} from ${agent}` }]
                };

                console.log(`📡 Sending request for agent: ${agent}`);
                const response = await fetch('http://localhost:4000/proxy/openai/v1/chat/completions', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        'Authorization': `Bearer ${apiKey}`,
                        'X-govrix-scout-Agent-Id': agent
                    },
                    body: JSON.stringify(payload)
                });

                const data = await response.json();

                if (response.ok) {
                    console.log(`✅ [${agent}] Response received successfully.`);
                } else {
                    console.error(`❌ [${agent}] Error:`, data);
                }

                // Add a small delay between requests
                await new Promise(r => setTimeout(r, 500));
            } catch (err) {
                console.error(`🔥 Failed to run request for ${agent}:`, err.message);
            }
        }
    }

    console.log('🎉 Traffic simulation complete!');
}

runTest();
