"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import Image from "next/image";
import { TrendingUp, ArrowRight, Sparkles } from "lucide-react";
import { Logo } from "@/components/Logo";

export default function LandingPage() {
  const [ready, setReady] = useState(false);
  useEffect(() => { setReady(true); }, []);

  return (
    <div className="landing-page" style={{ opacity: ready ? 1 : 0, transition: 'opacity 0.15s ease-in' }}>
      {/* Background Effects */}
      <div className="bg-effects">
        <div className="gradient-orb gradient-orb-1" />
        <div className="gradient-orb gradient-orb-2" />
        <div className="gradient-orb gradient-orb-3" />
        <div className="noise-overlay" />
      </div>

      {/* Navigation */}
      <header className="nav-header">
        <div className="nav-container">
          <Link href="/" className="nav-logo" style={{ overflow: 'hidden', textDecoration: 'none' }}>
            <Logo height={96} variant="light" className="-my-4" />
          </Link>

          <nav className="nav-links">
            <Link href="#markets" className="nav-link">Markets</Link>
            <Link href="/pricing" className="nav-link">Pricing</Link>
          </nav>

          <div className="nav-actions">
            <Link href="/auth/login" className="nav-login">
              Login
            </Link>
            <a
              href="/markets"
              style={{
                fontSize: '0.9rem',
                fontWeight: 600,
                color: '#0a0f1a',
                background: 'linear-gradient(135deg, #0ea5e9, #06b6d4)',
                padding: '0.6rem 1.25rem',
                borderRadius: '8px',
                textDecoration: 'none',
                boxShadow: '0 4px 15px rgba(14, 165, 233, 0.3)',
              }}
            >
              Try for free
            </a>
          </div>
        </div>
      </header>

      {/* Hero Section */}
      <section className="hero">
        <div className="hero-content">
          {/* Main Headline */}
          <h1 className="hero-title">
            Find Arbitrage Across
            <br />
            <span className="highlight">Prediction Markets.</span>
          </h1>

          {/* Subtitle */}
          <p className="hero-subtitle">
            Scan Kalshi, Polymarket, and more in real-time. Spot price discrepancies. Lock in risk-free profit with math, not luck.
          </p>

          {/* Try Now CTA */}
          <a
            href="/markets"
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '0.75rem',
              padding: '1rem 2.5rem',
              background: 'linear-gradient(135deg, #0ea5e9, #06b6d4)',
              borderRadius: '12px',
              fontSize: '1.1rem',
              fontWeight: 700,
              color: '#0a0f1a',
              textDecoration: 'none',
              boxShadow: '0 8px 25px rgba(14, 165, 233, 0.35)',
              marginBottom: '3rem',
              transition: 'all 0.25s',
            }}
          >
            <span>Try Now</span>
            <ArrowRight style={{ width: 20, height: 20 }} />
          </a>

          {/* Platform Logos */}
          <div className="platform-logos">
            <span className="platforms-label">Scanning odds across</span>
            <div className="logos-row">
              <div className="platform-logo-item">
                <Image src="/kalshi-logo-v2.png" alt="Kalshi" width={32} height={32} className="platform-img" />
                <span>Kalshi</span>
              </div>
              <div className="logo-divider" />
              <div className="platform-logo-item">
                <Image src="/polymarket-logo.png" alt="Polymarket" width={32} height={32} className="platform-img" />
                <span>Polymarket</span>
              </div>
              <div className="logo-divider" />
              <div className="platform-logo-item coming-soon">
                <Image src="/predictit-logo.svg" alt="PredictIt" width={32} height={32} style={{ borderRadius: '6px' }} />
                <span>PredictIt</span>
                <span className="soon-badge">Soon</span>
              </div>
              <div className="logo-divider" />
              <div className="platform-logo-item coming-soon">
                <Image src="/metaculus-logo.svg" alt="Metaculus" width={32} height={32} style={{ borderRadius: '6px' }} />
                <span>Metaculus</span>
                <span className="soon-badge">Soon</span>
              </div>
            </div>
          </div>
        </div>

        {/* Live Arb Finder Widget */}
        <div className="live-widget">
          <div className="widget-header">
            <div className="widget-pulse" />
            <span className="widget-title">Live Arb Scanner</span>
          </div>
          <div className="widget-body">
            {/* Arb Opportunity 1 */}
            <div className="widget-opp">
              <div className="opp-top">
                <span className="opp-pct">+3.2%</span>
                <span className="opp-event">Will BTC hit $150k by June?</span>
              </div>
              <div className="opp-legs">
                <div className="opp-leg">
                  <Image src="/kalshi-logo-v2.png" alt="Kalshi" width={20} height={20} className="leg-logo" />
                  <span className="leg-side yes">YES 42¢</span>
                </div>
                <div className="opp-leg">
                  <Image src="/polymarket-logo.png" alt="Polymarket" width={20} height={20} className="leg-logo" />
                  <span className="leg-side no">NO 55¢</span>
                </div>
              </div>
            </div>

            {/* Arb Opportunity 2 */}
            <div className="widget-opp">
              <div className="opp-top">
                <span className="opp-pct">+1.8%</span>
                <span className="opp-event">Fed rate cut in March?</span>
              </div>
              <div className="opp-legs">
                <div className="opp-leg">
                  <Image src="/polymarket-logo.png" alt="Polymarket" width={20} height={20} className="leg-logo" />
                  <span className="leg-side yes">YES 67¢</span>
                </div>
                <div className="opp-leg">
                  <Image src="/kalshi-logo-v2.png" alt="Kalshi" width={20} height={20} className="leg-logo" />
                  <span className="leg-side no">NO 31¢</span>
                </div>
              </div>
            </div>

            {/* Arb Opportunity 3 */}
            <div className="widget-opp">
              <div className="opp-top">
                <span className="opp-pct">+5.1%</span>
                <span className="opp-event">Oscar Best Picture winner?</span>
              </div>
              <div className="opp-legs">
                <div className="opp-leg">
                  <Image src="/kalshi-logo-v2.png" alt="Kalshi" width={20} height={20} className="leg-logo" />
                  <span className="leg-side yes">YES 38¢</span>
                </div>
                <div className="opp-leg">
                  <Image src="/polymarket-logo.png" alt="Polymarket" width={20} height={20} className="leg-logo" />
                  <span className="leg-side no">NO 57¢</span>
                </div>
              </div>
            </div>
          </div>
          <a
            href="/markets"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '0.5rem',
              padding: '0.85rem',
              borderTop: '1px solid rgba(255, 255, 255, 0.06)',
              fontSize: '0.85rem',
              fontWeight: 600,
              color: '#0ea5e9',
              textDecoration: 'none',
              transition: 'background 0.2s',
            }}
          >
            View all opportunities <ArrowRight style={{ width: 16, height: 16 }} />
          </a>
        </div>
      </section>

      {/* Dashboard Preview */}
      <section className="dashboard-preview" id="markets">
        <div className="preview-container">
          <div className="desktop-preview">
            <div className="browser-chrome">
              <div className="browser-dots">
                <span className="dot red" />
                <span className="dot yellow" />
                <span className="dot green" />
              </div>
              <div className="browser-tabs">
                <div className="tab active">
                  <TrendingUp className="tab-icon" />
                  <span>ARBIAGENT</span>
                </div>
                <div className="tab">
                  <Sparkles className="tab-icon" />
                  <span>PREDICTIONS</span>
                </div>
              </div>
            </div>

            <div className="dashboard-content">
              <div className="dash-sidebar">
                <div className="dash-nav-item active">
                  <div className="nav-dot" />
                  <span>Arbitrage</span>
                </div>
                <div className="dash-nav-item">
                  <div className="nav-dot" />
                  <span>Markets</span>
                </div>
                <div className="dash-nav-item">
                  <div className="nav-dot" />
                  <span>Bet Tracker</span>
                </div>
              </div>

              <div className="dash-main">
                <div className="dash-header">
                  <h4>Arbitrage Bets</h4>
                  <div className="filter-pills">
                    <span className="pill active">Pre-Match</span>
                    <span className="pill">500</span>
                    <span className="pill">Live</span>
                  </div>
                </div>

                <div className="dash-table">
                  <div className="table-header-row">
                    <span>ARB %</span>
                    <span>EVENT</span>
                    <span>BET NAME</span>
                    <span>PLATFORMS</span>
                  </div>

                  <div className="table-row highlight-row">
                    <span className="arb-pct">2.93%</span>
                    <div className="event-info">
                      <span className="event-date">Sun, Dec 8 at 4:25 PM</span>
                      <span className="event-name">Buffalo Bills vs Los Angeles Rams</span>
                      <span className="event-league">Football | NFL</span>
                    </div>
                    <span className="bet-name">Player Rushing Yards</span>
                    <div className="bet-books">
                      <div className="book-row">
                        <Image src="/kalshi-logo-v2.png" alt="Kalshi" width={18} height={18} className="book-logo" />
                        <span>Over 49.5</span>
                        <span className="odds">-180</span>
                        <span className="bet-btn cyan">BET ➚</span>
                      </div>
                      <div className="book-row">
                        <Image src="/polymarket-logo.png" alt="Polymarket" width={18} height={18} className="book-logo" />
                        <span>Under 49.5</span>
                        <span className="odds">+205</span>
                        <span className="bet-btn teal">BET ➚</span>
                      </div>
                    </div>
                  </div>

                  <div className="table-row">
                    <span className="arb-pct">1.47%</span>
                    <div className="event-info">
                      <span className="event-date">Mon, Dec 9 at 1:00 PM</span>
                      <span className="event-name">Fed Rate Decision - March 2026</span>
                      <span className="event-league">Economics | Federal Reserve</span>
                    </div>
                    <span className="bet-name">Rate Cut 25bps</span>
                    <div className="bet-books">
                      <div className="book-row">
                        <Image src="/polymarket-logo.png" alt="Polymarket" width={18} height={18} className="book-logo" />
                        <span>YES</span>
                        <span className="odds">67¢</span>
                        <span className="bet-btn cyan">BET ➚</span>
                      </div>
                      <div className="book-row">
                        <Image src="/kalshi-logo-v2.png" alt="Kalshi" width={18} height={18} className="book-logo" />
                        <span>NO</span>
                        <span className="odds">31¢</span>
                        <span className="bet-btn teal">BET ➚</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="footer">
        <div className="footer-container">
          <div className="footer-logo" style={{ display: 'flex', alignItems: 'center', overflow: 'hidden' }}>
            <Logo height={72} variant="light" className="-my-3" />
          </div>
          <p className="footer-copy">© 2026 ArbiAgent. All rights reserved.</p>
        </div>
      </footer>

      <style jsx>{`
        .landing-page {
          min-height: 100vh;
          background: linear-gradient(180deg, #0a0f1a 0%, #0d1321 50%, #111827 100%);
          color: #fff;
          font-family: 'DM Sans', -apple-system, BlinkMacSystemFont, sans-serif;
          position: relative;
          overflow-x: hidden;
        }

        .bg-effects {
          position: fixed;
          inset: 0;
          pointer-events: none;
          z-index: 0;
        }

        .gradient-orb {
          position: absolute;
          border-radius: 50%;
          filter: blur(120px);
          opacity: 0.35;
        }

        .gradient-orb-1 {
          width: 600px;
          height: 600px;
          background: radial-gradient(circle, #0ea5e9 0%, transparent 70%);
          top: -200px;
          right: -100px;
        }

        .gradient-orb-2 {
          width: 500px;
          height: 500px;
          background: radial-gradient(circle, #06b6d4 0%, transparent 70%);
          bottom: 20%;
          left: -150px;
        }

        .gradient-orb-3 {
          width: 400px;
          height: 400px;
          background: radial-gradient(circle, #0284c7 0%, transparent 70%);
          top: 50%;
          right: 20%;
          opacity: 0.15;
        }

        .noise-overlay {
          position: absolute;
          inset: 0;
          background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 400 400' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
          opacity: 0.03;
        }

        /* Navigation */
        .nav-header {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          z-index: 100;
          padding: 1rem 2rem;
          background: rgba(10, 15, 26, 0.8);
          backdrop-filter: blur(16px);
          border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        }

        .nav-container {
          max-width: 1400px;
          margin: 0 auto;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }

        .nav-logo {
          display: flex;
          align-items: center;
          gap: 0.75rem;
        }

        .nav-links {
          display: flex;
          align-items: center;
          gap: 2.5rem;
        }

        .nav-link {
          font-size: 0.9rem;
          font-weight: 500;
          color: rgba(255, 255, 255, 0.7);
          text-decoration: none;
          transition: color 0.2s;
        }

        .nav-link:hover {
          color: #fff;
        }

        .nav-actions {
          display: flex;
          align-items: center;
          gap: 1rem;
        }

        .nav-login {
          font-size: 0.9rem;
          font-weight: 500;
          color: rgba(255, 255, 255, 0.7);
          text-decoration: none;
          transition: color 0.2s;
        }

        .nav-login:hover { color: #fff; }

        .nav-cta {
          font-size: 0.9rem;
          font-weight: 600;
          color: #0a0f1a;
          background: linear-gradient(135deg, #0ea5e9, #06b6d4);
          padding: 0.6rem 1.25rem;
          border-radius: 8px;
          text-decoration: none;
          transition: all 0.2s;
          box-shadow: 0 4px 15px rgba(14, 165, 233, 0.3);
        }

        .nav-cta:hover {
          transform: translateY(-1px);
          box-shadow: 0 6px 20px rgba(14, 165, 233, 0.4);
        }

        /* Hero */
        .hero {
          position: relative;
          z-index: 1;
          padding: 9rem 2rem 4rem;
          max-width: 1400px;
          margin: 0 auto;
          display: grid;
          grid-template-columns: 1fr 400px;
          gap: 3rem;
          align-items: start;
        }

        .hero-content {
          padding-top: 1rem;
        }

        .hero-title {
          font-size: 3.75rem;
          font-weight: 700;
          line-height: 1.08;
          letter-spacing: -0.03em;
          margin-bottom: 1.25rem;
        }

        .highlight {
          background: linear-gradient(135deg, #0ea5e9, #06b6d4);
          -webkit-background-clip: text;
          -webkit-text-fill-color: transparent;
          background-clip: text;
        }

        .hero-subtitle {
          font-size: 1.15rem;
          color: rgba(255, 255, 255, 0.55);
          line-height: 1.6;
          margin-bottom: 2rem;
          max-width: 520px;
        }

        /* Try Now CTA */
        .try-now-btn {
          display: inline-flex;
          align-items: center;
          gap: 0.75rem;
          padding: 1rem 2.5rem;
          background: linear-gradient(135deg, #0ea5e9, #06b6d4);
          border-radius: 12px;
          font-size: 1.1rem;
          font-weight: 700;
          color: #0a0f1a;
          text-decoration: none;
          transition: all 0.25s;
          box-shadow: 0 8px 25px rgba(14, 165, 233, 0.35);
          margin-bottom: 3rem;
        }

        .try-now-btn:hover {
          transform: translateY(-2px);
          box-shadow: 0 12px 35px rgba(14, 165, 233, 0.45);
        }

        .try-arrow {
          width: 20px;
          height: 20px;
        }

        /* Platform Logos */
        .platform-logos {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
        }

        .platforms-label {
          font-size: 0.8rem;
          color: rgba(255, 255, 255, 0.35);
          text-transform: uppercase;
          letter-spacing: 0.1em;
          font-weight: 600;
        }

        .logos-row {
          display: flex;
          align-items: center;
          gap: 1.25rem;
        }

        .platform-logo-item {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 0.9rem;
          color: rgba(255, 255, 255, 0.7);
          font-weight: 500;
        }

        .platform-logo-item.coming-soon {
          opacity: 0.45;
        }

        .logo-divider {
          width: 1px;
          height: 20px;
          background: rgba(255, 255, 255, 0.15);
        }

        .placeholder-logo {
          width: 32px;
          height: 32px;
          border-radius: 8px;
          background: rgba(255, 255, 255, 0.1);
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 0.8rem;
          font-weight: 700;
          color: rgba(255, 255, 255, 0.5);
        }

        .soon-badge {
          font-size: 0.6rem;
          padding: 0.15rem 0.4rem;
          background: rgba(14, 165, 233, 0.15);
          color: #0ea5e9;
          border-radius: 4px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.05em;
        }

        /* Live Widget */
        .live-widget {
          background: rgba(255, 255, 255, 0.03);
          border: 1px solid rgba(255, 255, 255, 0.08);
          border-radius: 16px;
          overflow: hidden;
          box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
          margin-top: 1rem;
        }

        .widget-header {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          padding: 1rem 1.25rem;
          border-bottom: 1px solid rgba(255, 255, 255, 0.06);
          background: rgba(255, 255, 255, 0.02);
        }

        .widget-pulse {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background: #10b981;
          box-shadow: 0 0 8px #10b981;
          animation: pulse 2s infinite;
        }

        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }

        .widget-title {
          font-size: 0.85rem;
          font-weight: 600;
          color: rgba(255, 255, 255, 0.8);
        }

        .widget-body {
          padding: 0.5rem;
        }

        .widget-opp {
          padding: 0.75rem;
          border-radius: 10px;
          margin-bottom: 0.25rem;
          transition: background 0.2s;
        }

        .widget-opp:hover {
          background: rgba(255, 255, 255, 0.03);
        }

        .opp-top {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          margin-bottom: 0.5rem;
        }

        .opp-pct {
          font-size: 0.85rem;
          font-weight: 700;
          color: #10b981;
          background: rgba(16, 185, 129, 0.1);
          padding: 0.15rem 0.5rem;
          border-radius: 4px;
        }

        .opp-event {
          font-size: 0.85rem;
          color: rgba(255, 255, 255, 0.85);
          font-weight: 500;
        }

        .opp-legs {
          display: flex;
          gap: 0.75rem;
          padding-left: 0.25rem;
        }

        .opp-leg {
          display: flex;
          align-items: center;
          gap: 0.4rem;
        }

        .leg-side {
          font-size: 0.75rem;
          font-weight: 600;
          padding: 0.15rem 0.5rem;
          border-radius: 4px;
        }

        .leg-side.yes {
          background: rgba(14, 165, 233, 0.15);
          color: #38bdf8;
        }

        .leg-side.no {
          background: rgba(239, 68, 68, 0.15);
          color: #f87171;
        }

        .widget-cta {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 0.5rem;
          padding: 0.85rem;
          border-top: 1px solid rgba(255, 255, 255, 0.06);
          font-size: 0.85rem;
          font-weight: 600;
          color: #0ea5e9;
          text-decoration: none;
          transition: background 0.2s;
        }

        .widget-cta:hover {
          background: rgba(14, 165, 233, 0.05);
        }

        .widget-arrow {
          width: 16px;
          height: 16px;
        }

        /* Dashboard Preview */
        .dashboard-preview {
          position: relative;
          z-index: 1;
          padding: 2rem 2rem 6rem;
          max-width: 1400px;
          margin: 0 auto;
        }

        .preview-container {
          position: relative;
        }

        .desktop-preview {
          background: #0d1321;
          border: 1px solid rgba(255, 255, 255, 0.1);
          border-radius: 16px;
          overflow: hidden;
          box-shadow: 0 25px 60px -12px rgba(0, 0, 0, 0.6);
        }

        .browser-chrome {
          display: flex;
          align-items: center;
          gap: 1rem;
          padding: 0.75rem 1rem;
          background: rgba(255, 255, 255, 0.03);
          border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }

        .browser-dots {
          display: flex;
          gap: 6px;
        }

        .dot {
          width: 10px;
          height: 10px;
          border-radius: 50%;
        }
        .dot.red { background: #ef4444; }
        .dot.yellow { background: #f59e0b; }
        .dot.green { background: #10b981; }

        .browser-tabs {
          display: flex;
          gap: 0.5rem;
        }

        .tab {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          background: rgba(255, 255, 255, 0.05);
          border-radius: 6px;
          font-size: 0.75rem;
          font-weight: 600;
          color: rgba(255, 255, 255, 0.5);
        }

        .tab.active {
          background: rgba(14, 165, 233, 0.2);
          color: #0ea5e9;
        }

        .tab-icon { width: 14px; height: 14px; }

        .dashboard-content { display: flex; }

        .dash-sidebar {
          width: 180px;
          padding: 1rem;
          background: rgba(255, 255, 255, 0.02);
          border-right: 1px solid rgba(255, 255, 255, 0.05);
        }

        .dash-nav-item {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          padding: 0.75rem;
          border-radius: 8px;
          font-size: 0.85rem;
          color: rgba(255, 255, 255, 0.5);
          margin-bottom: 0.25rem;
        }

        .dash-nav-item.active {
          background: rgba(14, 165, 233, 0.15);
          color: #0ea5e9;
        }

        .nav-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: currentColor;
          opacity: 0.6;
        }

        .dash-main {
          flex: 1;
          padding: 1rem 1.25rem;
        }

        .dash-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          margin-bottom: 1rem;
        }

        .dash-header h4 {
          font-size: 1rem;
          font-weight: 600;
        }

        .filter-pills { display: flex; gap: 0.5rem; }

        .pill {
          padding: 0.35rem 0.75rem;
          background: rgba(255, 255, 255, 0.05);
          border-radius: 6px;
          font-size: 0.75rem;
          color: rgba(255, 255, 255, 0.5);
        }

        .pill.active {
          background: rgba(14, 165, 233, 0.2);
          color: #0ea5e9;
        }

        .dash-table {
          background: rgba(255, 255, 255, 0.02);
          border-radius: 8px;
          overflow: hidden;
        }

        .table-header-row {
          display: grid;
          grid-template-columns: 80px 1fr 150px 220px;
          gap: 1rem;
          padding: 0.75rem 1rem;
          font-size: 0.7rem;
          font-weight: 600;
          color: rgba(255, 255, 255, 0.35);
          text-transform: uppercase;
          letter-spacing: 0.06em;
          border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }

        .table-row {
          display: grid;
          grid-template-columns: 80px 1fr 150px 220px;
          gap: 1rem;
          padding: 1rem;
          align-items: start;
          border-bottom: 1px solid rgba(255, 255, 255, 0.03);
        }

        .highlight-row {
          background: rgba(14, 165, 233, 0.04);
          border-left: 3px solid #0ea5e9;
        }

        .arb-pct {
          color: #10b981;
          font-weight: 700;
          font-size: 0.9rem;
        }

        .event-info {
          display: flex;
          flex-direction: column;
          gap: 0.2rem;
        }

        .event-date {
          font-size: 0.7rem;
          color: rgba(255, 255, 255, 0.35);
        }

        .event-name {
          font-size: 0.85rem;
          font-weight: 600;
        }

        .event-league {
          font-size: 0.7rem;
          color: rgba(255, 255, 255, 0.35);
        }

        .bet-name {
          font-size: 0.85rem;
          color: rgba(255, 255, 255, 0.6);
          padding-top: 0.1rem;
        }

        .bet-books {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .book-row {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 0.78rem;
        }

        .odds {
          color: rgba(255, 255, 255, 0.45);
          margin-left: auto;
        }

        .bet-btn {
          padding: 0.2rem 0.5rem;
          border-radius: 4px;
          font-size: 0.65rem;
          font-weight: 700;
        }

        .bet-btn.cyan {
          background: rgba(14, 165, 233, 0.2);
          color: #0ea5e9;
        }

        .bet-btn.teal {
          background: rgba(6, 182, 212, 0.2);
          color: #06b6d4;
        }

        /* Footer */
        .footer {
          position: relative;
          z-index: 1;
          padding: 2rem;
          border-top: 1px solid rgba(255, 255, 255, 0.05);
        }

        .footer-container {
          max-width: 1200px;
          margin: 0 auto;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }

        .footer-logo {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-weight: 700;
        }

        .footer-copy {
          font-size: 0.85rem;
          color: rgba(255, 255, 255, 0.35);
        }

        /* Responsive */
        @media (max-width: 1024px) {
          .hero {
            grid-template-columns: 1fr;
            gap: 2rem;
          }
          .live-widget {
            max-width: 450px;
          }
          .table-header-row,
          .table-row {
            grid-template-columns: 60px 1fr 120px;
          }
          .bet-books { display: none; }
        }

        @media (max-width: 768px) {
          .nav-links { display: none; }
          .hero-title { font-size: 2.5rem; }
          .logos-row { flex-wrap: wrap; gap: 0.75rem; }
          .logo-divider { display: none; }
          .dash-sidebar { display: none; }
          .table-header-row,
          .table-row {
            grid-template-columns: 60px 1fr;
          }
          .bet-name { display: none; }
        }
      `}</style>
    </div>
  );
}
