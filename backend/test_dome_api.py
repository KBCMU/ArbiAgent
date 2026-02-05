"""
Test script for DomeAPI integration.
Run this to verify the API is working correctly.

Usage:
    python backend/test_dome_api.py
"""
import asyncio
from backend.services import get_dome_client


async def test_api():
    """Test DomeAPI endpoints."""
    client = get_dome_client()
    
    print("🧪 Testing DomeAPI Integration\n")
    
    # Test 1: Get markets
    print("1️⃣ Fetching open markets (limit 3)...")
    try:
        response = await client.get_polymarket_markets(limit=3, status="open")
        print(f"   ✅ Found {len(response.markets)} markets")
        print(f"   📊 Total available: {response.pagination.total}")
        if response.markets:
            market = response.markets[0]
            print(f"   📌 First market: {market.title}")
    except Exception as e:
        print(f"   ❌ Error: {e}")
    
    print()
    
    # Test 2: Search markets
    print("2️⃣ Searching for 'bitcoin' markets...")
    try:
        response = await client.search_markets(query="bitcoin", limit=3)
        print(f"   ✅ Found {len(response.markets)} markets")
        for market in response.markets:
            print(f"   📌 {market.title}")
    except Exception as e:
        print(f"   ❌ Error: {e}")
    
    print()
    
    # Test 3: Get specific market
    print("3️⃣ Fetching specific market by slug...")
    try:
        market = await client.get_market_by_slug(
            "will-gavin-newsom-win-the-2028-us-presidential-election"
        )
        if market:
            print(f"   ✅ Market found: {market.title}")
            print(f"   📊 Status: {market.status}")
            print(f"   💰 Total volume: ${market.volume_total:,.2f}")
            print(f"   🎯 Outcomes: {market.side_a.label} vs {market.side_b.label}")
        else:
            print("   ⚠️  Market not found")
    except Exception as e:
        print(f"   ❌ Error: {e}")
    
    print("\n✨ Tests complete!")


if __name__ == "__main__":
    asyncio.run(test_api())
