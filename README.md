# ArbiAgent

Prediction market arbitrage detection platform.

## Features

- 🔍 Real-time market data from Polymarket and Kalshi via DomeAPI
- 📊 Clean, data-dense dashboard interface
- 🎯 Arbitrage opportunity detection (coming soon)
- 🚀 FastAPI backend with async support
- ⚡ Next.js frontend with modern UI

## Tech Stack

### Backend
- **FastAPI** - High-performance Python API framework
- **DomeAPI** - Unified prediction market data
- **Pydantic** - Data validation and settings
- **httpx** - Async HTTP client

### Frontend
- **Next.js 15** - React framework with App Router
- **TypeScript** - Type-safe development
- **Tailwind CSS v4** - Utility-first styling
- **Framer Motion** - Smooth animations
- **Lucide React** - Icon library

## Getting Started

### Prerequisites
- Python 3.8+
- Node.js 18+
- DomeAPI key (get from [domeapi.io](https://domeapi.io))

### Backend Setup

1. Install dependencies:
```bash
pip install -r backend/requirements.txt
```

2. Configure environment:
```bash
# Create .env file
DOME_API_KEY=your_api_key_here
```

3. Run the server:
```bash
uvicorn backend.main:app --reload
```

Backend will be available at `http://localhost:8000`

### Frontend Setup

1. Install dependencies:
```bash
cd frontend
npm install
```

2. Run the dev server:
```bash
npm run dev
```

Frontend will be available at `http://localhost:3000`

## Project Structure

```
ArbiAgent/
├── backend/
│   ├── main.py              # FastAPI app
│   ├── config.py            # Configuration
│   ├── models/              # Data models
│   ├── services/            # DomeAPI client
│   └── routers/             # API endpoints
├── frontend/
│   ├── app/                 # Next.js pages
│   ├── components/          # React components
│   └── lib/                 # Utilities
├── directives/              # Agent SOPs
├── execution/               # Agent scripts
└── .tmp/                    # Temporary files
```

## API Endpoints

- `GET /` - Health check
- `GET /api/markets` - List markets with filters
- `GET /api/markets/search?q={query}` - Search markets
- `GET /api/markets/{slug}` - Get specific market

## Development

### Backend Testing
```bash
python backend/test_dome_api.py
```

### Frontend Development
The dashboard includes:
- Market data table with real-time updates
- Filtering by status (Open/Closed/All)
- Search functionality
- Responsive design

## Roadmap

- [x] DomeAPI integration
- [x] Dashboard UI
- [ ] Arbitrage detection engine
- [ ] Real-time WebSocket updates
- [ ] Kalshi integration
- [ ] Automated betting (future)

## License

MIT
