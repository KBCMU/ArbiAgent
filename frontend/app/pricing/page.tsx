"use client";

import Link from "next/link";
import { TrendingUp, ArrowRight, Check, Zap, Shield, Bot } from "lucide-react";
import { Logo } from "@/components/Logo";

export default function PricingPage() {
  return (
    <div className="pricing-page">
      {/* Background Effects */}
      <div className="bg-effects">
        <div className="gradient-orb gradient-orb-1" />
        <div className="gradient-orb gradient-orb-2" />
        <div className="noise-overlay" />
      </div>

      {/* Navigation */}
      <header className="nav-header">
        <div className="nav-container">
          <Link href="/" className="nav-logo" style={{ overflow: 'hidden', textDecoration: 'none', color: '#fff' }}>
            <Logo height={96} variant="light" className="-my-4" />
          </Link>

          <nav className="nav-links">
            <Link href="/pricing" className="nav-link active-link">Pricing</Link>
          </nav>

          <div className="nav-actions">
            <Link href="/auth/login" className="nav-login">Login</Link>
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

      {/* Hero */}
      <section className="pricing-hero">
        <h1 className="pricing-title">
          Pricing built for
          <br />
          <span className="highlight">every trader</span>
        </h1>
        <p className="pricing-subtitle">
          Start free. Upgrade when you&apos;re ready to unlock arbitrage opportunities and automated trading.
        </p>
      </section>

      {/* Pricing Cards */}
      <section className="pricing-grid">
        {/* Free Tier */}
        <div className="pricing-card">
          <div className="card-header">
            <div className="tier-icon free-icon">
              <Zap style={{ width: 24, height: 24 }} />
            </div>
            <h3 className="tier-name">Free</h3>
            <div className="tier-price">
              <span className="price-amount">$0</span>
              <span className="price-period">/month</span>
            </div>
            <p className="tier-desc">Explore prediction markets and track odds across platforms.</p>
          </div>
          <div className="card-features">
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#10b981', flexShrink: 0 }} />
              <span>Prediction Markets Dashboard</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#10b981', flexShrink: 0 }} />
              <span>Kalshi & Polymarket odds</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#10b981', flexShrink: 0 }} />
              <span>Real-time market data</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#10b981', flexShrink: 0 }} />
              <span>Basic market filtering</span>
            </div>
          </div>
          <a
            href="/markets"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '0.5rem',
              padding: '0.85rem 1.5rem',
              background: 'rgba(255, 255, 255, 0.08)',
              border: '1px solid rgba(255, 255, 255, 0.15)',
              borderRadius: '10px',
              fontSize: '0.95rem',
              fontWeight: 600,
              color: '#fff',
              textDecoration: 'none',
              margin: '0 1.5rem 1.5rem',
              transition: 'all 0.2s',
            }}
          >
            Get started <ArrowRight style={{ width: 16, height: 16 }} />
          </a>
        </div>

        {/* Pro Tier */}
        <div className="pricing-card featured">
          <div className="featured-badge">Most Popular</div>
          <div className="card-header">
            <div className="tier-icon pro-icon">
              <Shield style={{ width: 24, height: 24 }} />
            </div>
            <h3 className="tier-name">Pro</h3>
            <div className="tier-price">
              <span className="price-amount">$29</span>
              <span className="price-period">/month</span>
            </div>
            <p className="tier-desc">Unlock arbitrage detection across prediction markets and sportsbooks.</p>
          </div>
          <div className="card-features">
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#0ea5e9', flexShrink: 0 }} />
              <span>Everything in Free</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#0ea5e9', flexShrink: 0 }} />
              <span>Arbitrage opportunity scanner</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#0ea5e9', flexShrink: 0 }} />
              <span>Real-time arb alerts</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#0ea5e9', flexShrink: 0 }} />
              <span>Bet tracker & profit analytics</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#0ea5e9', flexShrink: 0 }} />
              <span>Priority support</span>
            </div>
          </div>
          <a
            href="/markets"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '0.5rem',
              padding: '0.85rem 1.5rem',
              background: 'linear-gradient(135deg, #0ea5e9, #06b6d4)',
              border: 'none',
              borderRadius: '10px',
              fontSize: '0.95rem',
              fontWeight: 700,
              color: '#0a0f1a',
              textDecoration: 'none',
              margin: '0 1.5rem 1.5rem',
              boxShadow: '0 8px 25px rgba(14, 165, 233, 0.35)',
              transition: 'all 0.2s',
            }}
          >
            Start Pro trial <ArrowRight style={{ width: 16, height: 16 }} />
          </a>
        </div>

        {/* Agent Tier */}
        <div className="pricing-card agent">
          <div className="card-header">
            <div className="tier-icon agent-icon">
              <Bot style={{ width: 24, height: 24 }} />
            </div>
            <h3 className="tier-name">Agent</h3>
            <div className="tier-price">
              <span className="price-amount coming-soon-price">Coming Soon</span>
            </div>
            <p className="tier-desc">Fully automated arbitrage. The agent places and executes trades for you.</p>
          </div>
          <div className="card-features">
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#f59e0b', flexShrink: 0 }} />
              <span>Everything in Pro</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#f59e0b', flexShrink: 0 }} />
              <span>Automated trade execution</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#f59e0b', flexShrink: 0 }} />
              <span>Multi-platform order placement</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#f59e0b', flexShrink: 0 }} />
              <span>Smart position sizing</span>
            </div>
            <div className="feature-item">
              <Check style={{ width: 18, height: 18, color: '#f59e0b', flexShrink: 0 }} />
              <span>24/7 autonomous scanning & trading</span>
            </div>
          </div>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '0.5rem',
              padding: '0.85rem 1.5rem',
              background: 'rgba(245, 158, 11, 0.1)',
              border: '1px solid rgba(245, 158, 11, 0.3)',
              borderRadius: '10px',
              fontSize: '0.95rem',
              fontWeight: 600,
              color: '#f59e0b',
              margin: '0 1.5rem 1.5rem',
              cursor: 'default',
            }}
          >
            Coming Soon
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="footer">
        <div className="footer-container">
          <div className="footer-logo">
            <TrendingUp style={{ width: 20, height: 20, color: '#0ea5e9' }} />
            <span>ArbiAgent</span>
          </div>
          <p className="footer-copy">© 2026 ArbiAgent. All rights reserved.</p>
        </div>
      </footer>

      <style jsx>{`
        .pricing-page {
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
          opacity: 0.3;
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

        .noise-overlay {
          position: absolute;
          inset: 0;
          background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 400 400' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
          opacity: 0.03;
        }

        /* Nav */
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
          text-decoration: none;
          color: #fff;
        }

        .logo-icon {
          width: 36px;
          height: 36px;
          background: linear-gradient(135deg, #0ea5e9, #06b6d4);
          border-radius: 10px;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .logo-text {
          font-size: 1.25rem;
          font-weight: 700;
          letter-spacing: -0.02em;
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

        .nav-link:hover { color: #fff; }

        .active-link {
          color: #0ea5e9 !important;
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

        /* Pricing Hero */
        .pricing-hero {
          position: relative;
          z-index: 1;
          text-align: center;
          padding: 10rem 2rem 3rem;
          max-width: 700px;
          margin: 0 auto;
        }

        .pricing-title {
          font-size: 3.5rem;
          font-weight: 700;
          line-height: 1.1;
          letter-spacing: -0.03em;
          margin-bottom: 1.25rem;
        }

        .highlight {
          background: linear-gradient(135deg, #0ea5e9, #06b6d4);
          -webkit-background-clip: text;
          -webkit-text-fill-color: transparent;
          background-clip: text;
        }

        .pricing-subtitle {
          font-size: 1.15rem;
          color: rgba(255, 255, 255, 0.5);
          line-height: 1.6;
        }

        /* Pricing Grid */
        .pricing-grid {
          position: relative;
          z-index: 1;
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 1.5rem;
          max-width: 1100px;
          margin: 0 auto;
          padding: 3rem 2rem 6rem;
          align-items: stretch;
        }

        .pricing-card {
          background: rgba(255, 255, 255, 0.03);
          border: 1px solid rgba(255, 255, 255, 0.08);
          border-radius: 20px;
          overflow: hidden;
          transition: transform 0.3s, box-shadow 0.3s;
          position: relative;
          display: flex;
          flex-direction: column;
        }

        .pricing-card:hover {
          transform: translateY(-4px);
        }

        .pricing-card.featured {
          background: rgba(14, 165, 233, 0.06);
          border: 1px solid rgba(14, 165, 233, 0.25);
          box-shadow: 0 20px 50px rgba(14, 165, 233, 0.15);
        }

        .pricing-card.featured:hover {
          box-shadow: 0 25px 60px rgba(14, 165, 233, 0.2);
        }

        .pricing-card.agent {
          background: rgba(245, 158, 11, 0.03);
          border: 1px solid rgba(245, 158, 11, 0.15);
        }

        .featured-badge {
          position: absolute;
          top: 0;
          left: 0;
          right: 0;
          text-align: center;
          padding: 0.4rem;
          background: linear-gradient(135deg, #0ea5e9, #06b6d4);
          font-size: 0.75rem;
          font-weight: 700;
          color: #0a0f1a;
          letter-spacing: 0.05em;
          text-transform: uppercase;
        }

        .card-header {
          padding: 2rem 1.5rem 1.5rem;
        }

        .featured .card-header {
          padding-top: 3rem;
        }

        .tier-icon {
          width: 48px;
          height: 48px;
          border-radius: 12px;
          display: flex;
          align-items: center;
          justify-content: center;
          margin-bottom: 1rem;
        }

        .free-icon {
          background: rgba(16, 185, 129, 0.15);
          color: #10b981;
        }

        .pro-icon {
          background: rgba(14, 165, 233, 0.15);
          color: #0ea5e9;
        }

        .agent-icon {
          background: rgba(245, 158, 11, 0.15);
          color: #f59e0b;
        }

        .tier-name {
          font-size: 1.25rem;
          font-weight: 700;
          margin-bottom: 0.75rem;
        }

        .tier-price {
          display: flex;
          align-items: baseline;
          gap: 0.25rem;
          margin-bottom: 0.75rem;
        }

        .price-amount {
          font-size: 3rem;
          font-weight: 800;
          letter-spacing: -0.03em;
          line-height: 1;
        }

        .coming-soon-price {
          font-size: 2.25rem;
          color: rgba(255, 255, 255, 0.5);
        }

        .price-period {
          font-size: 1rem;
          color: rgba(255, 255, 255, 0.4);
          font-weight: 500;
        }

        .tier-desc {
          font-size: 0.9rem;
          color: rgba(255, 255, 255, 0.45);
          line-height: 1.5;
        }

        .card-features {
          padding: 0 1.5rem 1.5rem;
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
          flex: 1;
        }

        .feature-item {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          font-size: 0.9rem;
          color: rgba(255, 255, 255, 0.75);
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

        @media (max-width: 900px) {
          .pricing-grid {
            grid-template-columns: 1fr;
            max-width: 420px;
          }
          .pricing-title { font-size: 2.5rem; }
        }

        @media (max-width: 768px) {
          .nav-links { display: none; }
        }
      `}</style>
    </div>
  );
}
