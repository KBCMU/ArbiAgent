"use client";

import { useEffect } from "react";

export default function LandingPage() {
  useEffect(() => {
    document.body.style.backgroundColor = "#ffffff";
    document.body.style.color = "#0a2540";

    const nav = document.getElementById("main-nav");
    const handleScroll = () => {
      if (window.scrollY > 20) nav?.classList.add("scrolled");
      else nav?.classList.remove("scrolled");
    };
    window.addEventListener("scroll", handleScroll);

    const revealObs = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) { e.target.classList.add("v"); revealObs.unobserve(e.target); }
        });
      },
      { threshold: 0.1 }
    );
    document.querySelectorAll(".ai").forEach((el) => revealObs.observe(el));

    const countObs = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (!e.isIntersecting) return;
          const el = e.target as HTMLElement;
          const target = parseInt(el.dataset.count || "0");
          const suffix = el.dataset.suffix || "";
          const dur = 1400;
          const t0 = performance.now();
          function step(now: number) {
            const p = Math.min((now - t0) / dur, 1);
            const easedP = 1 - Math.pow(1 - p, 3);
            el.textContent = Math.round(target * easedP).toLocaleString() + suffix;
            if (p < 1) requestAnimationFrame(step);
            else el.textContent = target.toLocaleString() + suffix;
          }
          requestAnimationFrame(step);
          countObs.unobserve(el);
        });
      },
      { threshold: 0.5 }
    );
    document.querySelectorAll("[data-count]").forEach((el) => countObs.observe(el));

    return () => {
      document.body.style.backgroundColor = "";
      document.body.style.color = "";
      window.removeEventListener("scroll", handleScroll);
      revealObs.disconnect();
      countObs.disconnect();
    };
  }, []);

  return (
    <div className="lp">
      {/* Animated wave hero background */}
      <div className="hero-bg">
        <svg viewBox="0 0 1440 800" preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg">
          <path className="hero-wave-1" d="M0,340 C360,260 720,420 1080,360 C1260,330 1380,380 1440,360 L1440,800 L0,800 Z" />
          <path className="hero-wave-2" d="M0,460 C240,400 540,520 900,460 C1140,420 1320,500 1440,480 L1440,800 L0,800 Z" />
          <path className="hero-wave-3" d="M0,580 C300,530 600,620 960,580 C1200,555 1350,610 1440,590 L1440,800 L0,800 Z" />
        </svg>
      </div>

      {/* Nav */}
      <nav id="main-nav">
        <div className="nav-inner">
          <a href="/" className="nav-logo">
            <img src="/arbiagent-logo-v2.png" alt="Arbiagent" />
          </a>
          <div className="nav-links">
            <a href="#products">Products</a>
            <a href="#who-its-for">Traders</a>
            <a href="#pricing">Pricing</a>
          </div>
          <div className="nav-actions">
            <a href="/auth/login" className="nav-ghost">Sign in</a>
            <a href="/markets" className="nav-cta">Start now</a>
          </div>
        </div>
      </nav>

      {/* Hero */}
      <section className="hero-section">
        <div className="hero-eyebrow-row">
          <span className="sparkle-dot">
            <svg viewBox="0 0 24 24" fill="none">
              <path d="M12 2 L14 10 L22 12 L14 14 L12 22 L10 14 L2 12 L10 10 Z" fill="#2563eb" />
            </svg>
          </span>
          <span>Trusted across prediction markets</span>
        </div>
        <h1 className="hero-title">
          Trading infrastructure for <em>prediction&nbsp;markets.</em>
        </h1>
        <p className="hero-subtitle">
          Arbitrage detection, +EV trade identification, and autonomous trading agents — everything serious traders need to find and capture edge on Kalshi, Polymarket and beyond.
        </p>
        <div className="hero-ctas">
          <a href="/markets" className="btn-primary">Start now</a>
          <a href="#products" className="btn-ghost">Explore products</a>
        </div>

        {/* Dashboard mock */}
        <div className="hero-dashboard">
          <div className="dash-topbar">
            <div className="dash-dots">
              <span></span><span></span><span></span>
            </div>
            <div className="dash-url">arbiagent.com/arbitrage</div>
            <div style={{ width: "46px" }}></div>
          </div>
          <div className="dash-body">
            <div className="dash-sidebar">
              <div className="dash-side-logo">
                <img src="/arbiagent-logo-v2.png" alt="" />
              </div>
              <div className="dash-nav-item active">
                <svg viewBox="0 0 24 24" fill="none"><path d="M3 17L9 11L13 15L21 7M21 7V12M21 7H16" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" /></svg>
                Arbitrage
              </div>
              <div className="dash-nav-item">
                <svg viewBox="0 0 24 24" fill="none"><circle cx="11" cy="11" r="7" stroke="currentColor" strokeWidth="2" /><path d="M21 21L16 16" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
                Markets
              </div>
              <div className="dash-nav-item">
                <svg viewBox="0 0 24 24" fill="none"><path d="M3 3v18h18" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /><path d="M7 14l4-4 3 3 5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" /></svg>
                Bet Tracker
              </div>
              <div className="dash-nav-item">
                <svg viewBox="0 0 24 24" fill="none"><path d="M12 2L3 7v6c0 5 4 9 9 10 5-1 9-5 9-10V7l-9-5z" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" /></svg>
                Agents
              </div>
            </div>
            <div className="dash-main">
              <div className="dash-head-row">
                <div className="dash-title">Arbitrage Opportunities</div>
                <div className="dash-filters">
                  <span className="dash-pill on">All</span>
                  <span className="dash-pill">Sports</span>
                  <span className="dash-pill">Politics</span>
                  <span className="dash-pill">Crypto</span>
                </div>
              </div>
              <div className="dash-stats">
                <div className="dash-stat"><div className="dash-stat-lbl">Opportunities</div><div className="dash-stat-val">247</div></div>
                <div className="dash-stat"><div className="dash-stat-lbl">Avg Margin</div><div className="dash-stat-val green">+3.2%</div></div>
                <div className="dash-stat"><div className="dash-stat-lbl">Best Margin</div><div className="dash-stat-val green">+6.8%</div></div>
                <div className="dash-stat"><div className="dash-stat-lbl">Platforms</div><div className="dash-stat-val">2</div></div>
              </div>
              <div className="dash-table">
                <div className="dash-thead">
                  <span>Margin</span><span>Event</span><span>Outcome</span><span>Buy Leg</span><span>Sell Leg</span>
                </div>
                {[
                  { margin: "+3.2%", name: "Will BTC hit $150k by June 2026?", cat: "Crypto", buy: { src: "/kalshi-logo-v2.png", price: "42¢" }, sell: { src: "/polymarket-logo.png", price: "55¢" } },
                  { margin: "+2.0%", name: "Fed Rate Cut Before May 2026", cat: "Economics", buy: { src: "/polymarket-logo.png", price: "67¢" }, sell: { src: "/kalshi-logo-v2.png", price: "31¢" } },
                  { margin: "+5.3%", name: "Trump Approval Above 50% in April", cat: "Politics", buy: { src: "/kalshi-logo-v2.png", price: "38¢" }, sell: { src: "/polymarket-logo.png", price: "57¢" } },
                  { margin: "+1.8%", name: "Lakers Make 2026 Playoffs", cat: "NBA", buy: { src: "/polymarket-logo.png", price: "71¢" }, sell: { src: "/kalshi-logo-v2.png", price: "26¢" } },
                ].map((row, i) => (
                  <div key={i} className="dash-trow">
                    <span className="dash-margin">{row.margin}</span>
                    <div>
                      <div className="dash-evname">{row.name}</div>
                      <div className="dash-evcat">{row.cat}</div>
                    </div>
                    <span className="dash-outcome">Yes / No</span>
                    <div className="dash-leg"><img src={row.buy.src} alt="" /><span className="dash-leg-price">{row.buy.price}</span></div>
                    <div className="dash-leg"><img src={row.sell.src} alt="" /><span className="dash-leg-price">{row.sell.price}</span></div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Platform logos strip */}
      <div className="logos-strip">
        <div className="logos-inner">
          <div className="logos-label">Scanning and trading across</div>
          <div className="logos-row">
            <div className="lg"><img src="/kalshi-logo-v2.png" alt="" /><span>Kalshi</span></div>
            <div className="lg"><img src="/polymarket-logo.png" alt="" /><span>Polymarket</span></div>
            <div className="lg soon"><img src="/predictit-logo.svg" alt="" /><span>PredictIt</span><span className="soon-chip">Soon</span></div>
            <div className="lg soon"><img src="/metaculus-logo.svg" alt="" /><span>Metaculus</span><span className="soon-chip">Soon</span></div>
          </div>
        </div>
      </div>

      {/* Bento product grid */}
      <section className="bento-section" id="products">
        <div className="bento-inner">
          <div className="section-head">
            <div className="section-eyebrow ai">Products</div>
            <h2 className="section-title ai d1">A complete trading toolkit, <em>built&nbsp;for edge.</em></h2>
            <p className="section-sub ai d2">From lock-in arbitrage to fully autonomous agents — Arbiagent gives you the full stack for serious prediction market trading.</p>
          </div>
          <div className="bento-grid">
            <div className="bento bento-tall-left ai">
              <div className="bento-head">01 — Arbitrage</div>
              <h3>Lock in risk-free profit.</h3>
              <p>When YES and NO prices across platforms sum to less than $1.00, we find it instantly. Place both legs and collect the spread — guaranteed, regardless of outcome.</p>
              <div className="bento-visual">
                <div className="viz-arb">
                  <div className="viz-arb-head"><span>Live Arbitrage</span><span style={{ color: "#367c53" }}>●</span></div>
                  {[
                    { pct: "+3.2%", ev: "BTC $150k by June", a: "/kalshi-logo-v2.png", b: "/polymarket-logo.png" },
                    { pct: "+2.0%", ev: "Fed Rate Cut May 2026", a: "/polymarket-logo.png", b: "/kalshi-logo-v2.png" },
                    { pct: "+5.3%", ev: "Trump Approval >50%", a: "/kalshi-logo-v2.png", b: "/polymarket-logo.png" },
                  ].map((r, i) => (
                    <div key={i} className="viz-arb-row">
                      <span className="viz-pill">{r.pct}</span>
                      <span className="viz-ev-text">{r.ev}</span>
                      <div className="viz-logo-pair">
                        <img src={r.a} alt="" />
                        <img src={r.b} alt="" />
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className="bento bento-wide bento-blue ai d1">
              <div className="bento-head">02 — +EV Trading</div>
              <h3>Find positive expected value.</h3>
              <p>Beyond arb — our models surface markets that are statistically mispriced relative to fair value.</p>
              <div className="bento-visual">
                <div className="viz-ev-grid">
                  <div className="ev-card">
                    <div className="ev-card-lbl">Market Price</div>
                    <div className="ev-card-row"><span>YES</span><span className="ev-num">42¢</span></div>
                  </div>
                  <div className="ev-card">
                    <div className="ev-card-lbl">Fair Value</div>
                    <div className="ev-card-row"><span>YES</span><span className="ev-num ev-green">51¢</span></div>
                  </div>
                </div>
              </div>
            </div>

            <div className="bento bento-wide bento-dark ai d2">
              <div className="bento-head">03 — Autonomous Agents</div>
              <h3>Deploy and go hands-free.</h3>
              <p>Configure a trading agent for any market category. It scans, decides, executes, and reports — 24/7.</p>
              <div className="bento-visual">
                <div className="viz-agent">
                  <div><span className="c">$</span> <span className="k">agent</span> deploy --market=politics</div>
                  <div><span className="c"># Scanning 1,247 markets…</span></div>
                  <div><span className="s">✓</span> Found +EV edge on 3 markets</div>
                  <div><span className="s">✓</span> Placed 2 positions · $420 risk</div>
                  <div><span className="k">status</span>: <span className="s">live</span><span className="viz-agent-blink"></span></div>
                </div>
              </div>
            </div>

            <div className="bento bento-reg-3 ai">
              <div className="bento-head">Markets</div>
              <h3>One dashboard, every market.</h3>
              <p>Live odds from every major prediction platform, unified into a single normalized view.</p>
            </div>

            <div className="bento bento-reg-3 bento-green ai d1">
              <div className="bento-head">Analytics</div>
              <h3>Know your edge in real time.</h3>
              <p>Track every position, measure P&amp;L by strategy, and analyze which markets work best for you.</p>
            </div>
          </div>
        </div>
      </section>

      {/* Who it's for */}
      <section className="personas" id="who-its-for">
        <div className="personas-inner">
          <div className="section-head">
            <div className="section-eyebrow ai">Who it&apos;s for</div>
            <h2 className="section-title ai d1">Built for every kind of <em>trader.</em></h2>
            <p className="section-sub ai d2">Whether you&apos;re just spotting odds or running autonomous strategies, Arbiagent meets you where you are.</p>
          </div>
          <div className="persona-grid">
            {[
              {
                icon: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="11" cy="11" r="7" /><path d="M21 21l-5-5" /></svg>,
                title: "The researcher",
                desc: "Explore live odds across Kalshi and Polymarket. Compare markets, watch spreads, and build conviction before you act.",
                cta: "Start with Markets",
                delay: "",
              },
              {
                icon: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 17l6-6 4 4 8-8" /><path d="M21 7v5h-5" /></svg>,
                title: "The arb trader",
                desc: "Capture cross-platform price gaps the moment they appear. Use our scanner, our alerts, and our execution playbook.",
                cta: "Explore Arbitrage",
                delay: "d1",
              },
              {
                icon: <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 2l3 7h7l-5.5 4.5 2 7L12 16l-6.5 4.5 2-7L2 9h7z" /></svg>,
                title: "The autonomous trader",
                desc: "Deploy agents per market vertical. Your strategies, your risk parameters, running 24/7 without you touching a keyboard.",
                cta: "Meet Agents",
                delay: "d2",
              },
            ].map((p) => (
              <a key={p.title} href="#" className={`persona ai ${p.delay}`}>
                <div className="persona-icon">{p.icon}</div>
                <h4>{p.title}</h4>
                <p>{p.desc}</p>
                <span className="persona-cta">{p.cta}</span>
              </a>
            ))}
          </div>
        </div>
      </section>

      {/* Stats bar */}
      <section className="stats-bar">
        <div className="stats-inner">
          <h2 className="stats-title">The <em>infrastructure layer</em> for prediction market trading.</h2>
          <div className="stats-grid">
            <div className="stat-item">
              <div className="stat-n" data-count="2">0</div>
              <div className="stat-l">Platforms integrated today — more coming soon</div>
            </div>
            <div className="stat-item">
              <div className="stat-n">&lt;2s</div>
              <div className="stat-l">Average arbitrage detection latency</div>
            </div>
            <div className="stat-item">
              <div className="stat-n">24/7</div>
              <div className="stat-l">Real-time scanning across every market</div>
            </div>
            <div className="stat-item">
              <div className="stat-n" data-count="1000" data-suffix="+">0</div>
              <div className="stat-l">Unique markets tracked across platforms</div>
            </div>
          </div>
        </div>
      </section>

      {/* Pricing */}
      <section className="pricing-section" id="pricing">
        <div className="pricing-inner">
          <div className="section-head" style={{ textAlign: "left" }}>
            <div className="section-eyebrow ai">Pricing</div>
            <h2 className="section-title ai d1">Pricing for <em>every level.</em></h2>
            <p className="section-sub ai d2">Start with the markets dashboard. Unlock arb and +EV tools when you&apos;re ready. Let agents trade for you when you want to go hands-free.</p>
          </div>
          <div className="pricing-grid">
            <div className="pc ai">
              <div className="pc-tier">Starter</div>
              <div className="pc-name">Free</div>
              <div className="pc-price"><span className="pc-num">$0</span><span className="pc-per">/month</span></div>
              <p className="pc-desc">Explore live markets across every major platform.</p>
              <ul className="pc-feats">
                {["Full markets dashboard", "Kalshi & Polymarket odds", "Real-time market data", "Bet tracker", "Basic filtering & search"].map((f) => (
                  <li key={f}><svg className="ck" viewBox="0 0 16 16"><path d="M3 8l3 3 7-7" /></svg>{f}</li>
                ))}
              </ul>
              <a href="/markets" className="pc-btn btn-deep">Get started</a>
            </div>
            <div className="pc featured ai d1">
              <div className="pc-tier">Most Popular</div>
              <div className="pc-name">Pro</div>
              <div className="pc-price"><span className="pc-num">$29</span><span className="pc-per">/month</span></div>
              <p className="pc-desc">Unlock arbitrage and +EV tools for serious traders.</p>
              <ul className="pc-feats">
                {["Everything in Starter", "Live arbitrage scanner", "+EV trade identification", "Real-time alerts", "Advanced P&L analytics", "Priority support"].map((f) => (
                  <li key={f}><svg className="ck" viewBox="0 0 16 16"><path d="M3 8l3 3 7-7" /></svg>{f}</li>
                ))}
              </ul>
              <a href="/markets" className="pc-btn btn-white">Start Pro trial</a>
            </div>
            <div className="pc coming ai d2">
              <div className="pc-tier">Coming Soon</div>
              <div className="pc-name">Agent</div>
              <div className="pc-price"><span className="pc-coming-lbl">Autonomous</span></div>
              <p className="pc-desc">Deploy agents that scan and trade for you.</p>
              <ul className="pc-feats">
                {["Everything in Pro", "Deploy agents per market", "Automated arb & +EV execution", "Smart position sizing", "24/7 autonomous operation"].map((f) => (
                  <li key={f}><svg className="ck" viewBox="0 0 16 16"><path d="M3 8l3 3 7-7" /></svg>{f}</li>
                ))}
              </ul>
              <div className="pc-btn btn-amber">Notify me</div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA band */}
      <section className="cta-band">
        <h2 className="ai">Ready to find your <em>edge?</em></h2>
        <p className="ai d1">Create an account and start exploring live markets in under a minute. No credit card required.</p>
        <div className="hero-ctas ai d2">
          <a href="/markets" className="btn-primary">Start now</a>
          <a href="#pricing" className="btn-ghost" style={{ background: "#f6f9fc" }}>See pricing</a>
        </div>
      </section>

      {/* Footer */}
      <footer>
        <div className="footer-top">
          <div className="footer-brand">
            <img src="/arbiagent-logo-v2.png" alt="Arbiagent" />
            <p className="footer-tagline">Trading infrastructure for prediction markets. Arbitrage, +EV trading, and autonomous agents in one toolkit.</p>
            <div className="footer-socials">
              <a href="#" aria-label="Twitter">
                <svg viewBox="0 0 24 24" fill="currentColor"><path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24h-6.643l-5.214-6.817L4.5 21.75H1.184l7.73-8.835L0.75 2.25h6.81l4.713 6.231z" /></svg>
              </a>
              <a href="#" aria-label="GitHub">
                <svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.57.1.78-.25.78-.55v-2.08c-3.2.7-3.87-1.3-3.87-1.3-.53-1.33-1.29-1.69-1.29-1.69-1.05-.72.08-.7.08-.7 1.17.08 1.78 1.2 1.78 1.2 1.04 1.78 2.71 1.26 3.38.97.1-.76.4-1.27.74-1.56-2.56-.3-5.26-1.28-5.26-5.69 0-1.26.45-2.28 1.18-3.08-.12-.29-.51-1.46.11-3.05 0 0 .97-.31 3.18 1.18a11.05 11.05 0 0 1 5.78 0c2.21-1.49 3.18-1.18 3.18-1.18.62 1.59.23 2.76.11 3.05.74.8 1.18 1.82 1.18 3.08 0 4.42-2.7 5.39-5.27 5.68.41.36.77 1.05.77 2.13v3.15c0 .31.21.66.79.55A11.5 11.5 0 0 0 23.5 12c0-6.35-5.15-11.5-11.5-11.5z" /></svg>
              </a>
              <a href="#" aria-label="LinkedIn">
                <svg viewBox="0 0 24 24" fill="currentColor"><path d="M20.45 20.45h-3.56v-5.57c0-1.33-.03-3.04-1.85-3.04-1.86 0-2.14 1.45-2.14 2.94v5.67H9.34V9h3.41v1.56h.05c.48-.9 1.64-1.85 3.37-1.85 3.6 0 4.27 2.37 4.27 5.45v6.29zM5.34 7.43a2.06 2.06 0 1 1 0-4.12 2.06 2.06 0 0 1 0 4.12zM7.12 20.45H3.56V9h3.56v11.45zM22.22 0H1.77C.79 0 0 .77 0 1.72v20.56C0 23.23.79 24 1.77 24h20.45c.98 0 1.78-.77 1.78-1.72V1.72C24 .77 23.2 0 22.22 0z" /></svg>
              </a>
            </div>
          </div>
          <div className="footer-col">
            <h5>Product</h5>
            <a href="/markets">Markets</a>
            <a href="/arbitrage">Arbitrage</a>
            <a href="#" className="dim">+EV Trading</a>
            <a href="/bet-tracker">Bet Tracker</a>
            <a href="#" className="dim">Agents (soon)</a>
          </div>
          <div className="footer-col">
            <h5>Platforms</h5>
            <a href="https://kalshi.com" target="_blank" rel="noopener">Kalshi</a>
            <a href="https://polymarket.com" target="_blank" rel="noopener">Polymarket</a>
            <a href="#" className="dim">PredictIt (soon)</a>
            <a href="#" className="dim">Metaculus (soon)</a>
          </div>
          <div className="footer-col">
            <h5>Company</h5>
            <a href="#">About</a>
            <a href="#">Blog</a>
            <a href="#">Careers</a>
            <a href="#">Contact</a>
          </div>
          <div className="footer-col">
            <h5>Account</h5>
            <a href="/pricing">Pricing</a>
            <a href="/auth/login">Sign in</a>
            <a href="/auth/signup">Sign up</a>
            <a href="#">Support</a>
          </div>
        </div>
        <div className="footer-bottom">
          <span className="footer-copy">© 2026 Arbiagent, Inc. All rights reserved.</span>
          <div className="footer-legal">
            <a href="#">Privacy</a>
            <a href="#">Terms</a>
            <a href="#">Cookies</a>
            <a href="#">Security</a>
          </div>
        </div>
        <div className="footer-disclaimer">
          Arbiagent is a market intelligence and trading tools platform. We do not accept or custody user funds. Trading prediction markets involves risk. Ensure you comply with your local laws before participating in any real-money prediction market.
        </div>
      </footer>

      <style jsx>{`
        /* ── Variables ── */
        .lp { --blue: #2563eb; --blue-hover: #1d4ed8; --blue-deep: #0a2540; --emerald: #367c53; --emerald-light: #d1fae5; --bg: #ffffff; --bg-gray: #f6f9fc; --bg-deep: #0a2540; --border: #e3e8ee; --border-sub: #eef1f5; --text: #0a2540; --text-soft: #3c4257; --muted: #425466; --dim: #8792a2; --font: var(--font-jakarta, 'Plus Jakarta Sans', -apple-system, sans-serif); --mono: var(--font-mono, 'JetBrains Mono', monospace); }

        /* ── Wave hero bg ── */
        .hero-bg { position: absolute; top: 0; left: 0; right: 0; height: 780px; z-index: 0; overflow: hidden; background: linear-gradient(180deg, #c7f1d3 0%, #a8d7f5 22%, #b4b4f8 48%, #f8c8d9 72%, #fbecd0 100%); clip-path: polygon(0 0, 100% 0, 100% 82%, 0 100%); }
        .hero-bg svg { position: absolute; top: 0; left: 0; width: 100%; height: 100%; filter: blur(2px); }
        .hero-wave-1 { fill: rgba(255,255,255,0.35); animation: wave-shift-1 16s ease-in-out infinite alternate; }
        .hero-wave-2 { fill: rgba(37,99,235,0.08); animation: wave-shift-2 22s ease-in-out infinite alternate; }
        .hero-wave-3 { fill: rgba(54,124,83,0.06); animation: wave-shift-3 28s ease-in-out infinite alternate; }
        @keyframes wave-shift-1 { 0% { transform: translate(0,0) scale(1); } 100% { transform: translate(-40px,20px) scale(1.05); } }
        @keyframes wave-shift-2 { 0% { transform: translate(0,0) scale(1); } 100% { transform: translate(30px,-15px) scale(1.03); } }
        @keyframes wave-shift-3 { 0% { transform: translate(0,0) scale(1); } 100% { transform: translate(-20px,25px) scale(1.06); } }

        /* ── Nav ── */
        nav { position: sticky; top: 0; z-index: 100; padding: 0 2rem; height: 74px; display: flex; align-items: center; justify-content: space-between; background: transparent; font-family: var(--font); }
        nav.scrolled { background: rgba(255,255,255,0.88); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border-bottom: 1px solid var(--border); }
        .nav-inner { max-width: 1280px; width: 100%; margin: 0 auto; display: flex; align-items: center; justify-content: space-between; }
        .nav-logo img { height: 46px; display: block; }
        .nav-links { display: flex; gap: 1.875rem; }
        .nav-links a { font-size: 0.9375rem; font-weight: 500; color: var(--text); text-decoration: none; transition: color 0.15s; padding: 0.5rem 0; }
        .nav-links a:hover { color: var(--blue); }
        .nav-actions { display: flex; align-items: center; gap: 0.75rem; }
        .nav-ghost { font-size: 0.9375rem; font-weight: 500; color: var(--text); text-decoration: none; transition: color 0.15s; }
        .nav-ghost:hover { color: var(--blue); }
        .nav-cta { font-size: 0.9375rem; font-weight: 600; color: #fff; background: var(--blue-deep); text-decoration: none; padding: 0.5rem 1rem; border-radius: 18px; transition: background 0.15s; display: inline-flex; align-items: center; gap: 0.25rem; }
        .nav-cta:hover { background: #173357; }
        .nav-cta::after { content: '→'; font-size: 0.85em; transition: transform 0.2s; }
        .nav-cta:hover::after { transform: translateX(2px); }

        /* ── Hero ── */
        .hero-section { position: relative; z-index: 1; padding: 4rem 2rem 5rem; max-width: 1280px; margin: 0 auto; font-family: var(--font); }
        .hero-eyebrow-row { display: flex; align-items: center; gap: 0.75rem; font-size: 0.8125rem; font-weight: 500; color: var(--text-soft); margin-bottom: 2rem; }
        .sparkle-dot { width: 24px; height: 24px; display: inline-flex; align-items: center; justify-content: center; }
        .sparkle-dot svg { width: 100%; height: 100%; }
        h1.hero-title { font-size: clamp(3rem, 6.2vw, 5.75rem); font-weight: 700; line-height: 1.02; letter-spacing: -0.035em; color: var(--text); max-width: 960px; margin-bottom: 2rem; }
        h1.hero-title em { font-style: italic; font-weight: 700; background: linear-gradient(135deg, #2563eb 0%, #367c53 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
        .hero-subtitle { font-size: 1.1875rem; color: var(--text-soft); line-height: 1.55; max-width: 640px; margin-bottom: 2.5rem; font-weight: 400; }
        .hero-ctas { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
        .btn-primary { display: inline-flex; align-items: center; gap: 0.4rem; padding: 0.75rem 1.25rem; background: var(--blue-deep); color: #fff; font-size: 0.9375rem; font-weight: 600; border-radius: 20px; text-decoration: none; transition: background 0.15s; font-family: var(--font); }
        .btn-primary:hover { background: #173357; }
        .btn-primary::after { content: '→'; transition: transform 0.2s; }
        .btn-primary:hover::after { transform: translateX(3px); }
        .btn-ghost { display: inline-flex; align-items: center; gap: 0.4rem; padding: 0.75rem 1.25rem; background: rgba(255,255,255,0.6); color: var(--text); font-size: 0.9375rem; font-weight: 600; border-radius: 20px; text-decoration: none; border: 1px solid rgba(10,37,64,0.08); transition: background 0.15s; backdrop-filter: blur(8px); font-family: var(--font); }
        .btn-ghost:hover { background: rgba(255,255,255,0.9); }

        /* ── Dashboard mock ── */
        .hero-dashboard { margin-top: 3rem; background: #fff; border-radius: 12px; box-shadow: 0 50px 100px -20px rgba(50,50,93,0.25), 0 30px 60px -30px rgba(0,0,0,0.3); overflow: hidden; border: 1px solid var(--border); max-width: 1120px; margin-left: auto; margin-right: auto; margin-top: 3rem; font-family: var(--font); }
        .dash-topbar { display: flex; align-items: center; gap: 0.75rem; padding: 0.75rem 1rem; border-bottom: 1px solid var(--border); background: #f9fafc; }
        .dash-dots { display: flex; gap: 0.4rem; }
        .dash-dots span { width: 11px; height: 11px; border-radius: 50%; background: #e3e8ee; }
        .dash-url { flex: 1; font-family: var(--mono); font-size: 0.78rem; color: var(--dim); background: #fff; border: 1px solid var(--border); padding: 0.3rem 0.75rem; border-radius: 6px; max-width: 380px; margin: 0 auto; }
        .dash-body { display: grid; grid-template-columns: 200px 1fr; min-height: 440px; }
        .dash-sidebar { background: #f9fafc; border-right: 1px solid var(--border); padding: 1rem 0.75rem; }
        .dash-side-logo img { height: 32px; margin: 0.25rem 0.5rem 1.25rem; }
        .dash-nav-item { display: flex; align-items: center; gap: 0.625rem; padding: 0.5rem 0.75rem; border-radius: 7px; font-size: 0.8125rem; font-weight: 500; color: var(--muted); margin-bottom: 0.125rem; cursor: pointer; }
        .dash-nav-item.active { background: #eff6ff; color: var(--blue); font-weight: 600; }
        .dash-nav-item svg { width: 14px; height: 14px; flex-shrink: 0; }
        .dash-main { padding: 1.25rem 1.5rem; }
        .dash-head-row { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; }
        .dash-title { font-size: 1.0625rem; font-weight: 700; letter-spacing: -0.02em; }
        .dash-filters { display: flex; gap: 0.4rem; }
        .dash-pill { font-size: 0.72rem; font-weight: 600; padding: 0.25rem 0.6rem; border-radius: 6px; background: #f1f5f9; color: var(--muted); }
        .dash-pill.on { background: #eff6ff; color: var(--blue); }
        .dash-stats { display: grid; grid-template-columns: repeat(4,1fr); gap: 0.75rem; margin-bottom: 1.25rem; }
        .dash-stat { background: #fff; border: 1px solid var(--border); border-radius: 8px; padding: 0.75rem 0.875rem; }
        .dash-stat-lbl { font-size: 0.68rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--dim); margin-bottom: 0.3rem; }
        .dash-stat-val { font-family: var(--mono); font-size: 1.125rem; font-weight: 700; color: var(--text); }
        .dash-stat-val.green { color: var(--emerald); }
        .dash-table { background: #fff; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
        .dash-thead, .dash-trow { display: grid; grid-template-columns: 60px 1fr 90px 110px 110px; gap: 0.75rem; padding: 0.55rem 0.875rem; align-items: center; }
        .dash-thead { background: #f9fafc; border-bottom: 1px solid var(--border); }
        .dash-thead span { font-size: 0.64rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--dim); }
        .dash-trow { border-bottom: 1px solid var(--border-sub); border-left: 3px solid var(--emerald); }
        .dash-trow:last-child { border-bottom: none; }
        .dash-margin { font-family: var(--mono); font-size: 0.72rem; font-weight: 700; color: #fff; background: var(--emerald); padding: 0.18rem 0.4rem; border-radius: 5px; text-align: center; }
        .dash-evname { font-size: 0.8125rem; font-weight: 600; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
        .dash-evcat { font-size: 0.68rem; color: var(--dim); font-weight: 500; margin-top: 0.1rem; }
        .dash-outcome { font-size: 0.72rem; font-weight: 600; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
        .dash-leg { display: flex; align-items: center; gap: 0.35rem; }
        .dash-leg img { width: 15px; height: 15px; border-radius: 3px; }
        .dash-leg-price { font-family: var(--mono); font-size: 0.78rem; font-weight: 700; }

        /* ── Logos strip ── */
        .logos-strip { padding: 3rem 2rem 2rem; background: var(--bg); position: relative; z-index: 2; font-family: var(--font); }
        .logos-inner { max-width: 1200px; margin: 0 auto; text-align: center; }
        .logos-label { font-size: 0.8125rem; font-weight: 500; color: var(--muted); margin-bottom: 1.75rem; }
        .logos-row { display: flex; justify-content: center; align-items: center; gap: 3.5rem; flex-wrap: wrap; }
        .lg { display: flex; align-items: center; gap: 0.5rem; color: var(--muted); font-weight: 600; font-size: 1.125rem; letter-spacing: -0.01em; }
        .lg img { height: 28px; width: auto; object-fit: contain; border-radius: 4px; }
        .lg.soon { opacity: 0.4; }
        .soon-chip { font-size: 0.6rem; font-weight: 700; padding: 0.1rem 0.4rem; background: #eff6ff; color: var(--blue); border-radius: 4px; text-transform: uppercase; letter-spacing: 0.05em; }

        /* ── Bento grid ── */
        .bento-section { padding: 6rem 2rem 5rem; background: var(--bg-gray); position: relative; font-family: var(--font); }
        .bento-inner { max-width: 1280px; margin: 0 auto; }
        .section-head { max-width: 720px; margin-bottom: 3rem; }
        .section-eyebrow { font-size: 0.8125rem; font-weight: 600; color: var(--blue); margin-bottom: 1rem; letter-spacing: 0.02em; }
        .section-title { font-size: clamp(2.25rem, 4.5vw, 3.5rem); font-weight: 700; line-height: 1.06; letter-spacing: -0.03em; color: var(--text); margin-bottom: 1rem; }
        .section-title em { font-style: italic; background: linear-gradient(135deg, #2563eb 0%, #367c53 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
        .section-sub { font-size: 1.125rem; color: var(--muted); line-height: 1.55; }
        .bento-grid { display: grid; grid-template-columns: repeat(6,1fr); grid-auto-rows: minmax(260px,auto); gap: 1.25rem; }
        .bento { background: #fff; border-radius: 16px; padding: 2rem; display: flex; flex-direction: column; position: relative; overflow: hidden; transition: transform 0.25s, box-shadow 0.25s; border: 1px solid var(--border-sub); }
        .bento:hover { transform: translateY(-4px); box-shadow: 0 24px 48px -20px rgba(50,50,93,0.15); }
        .bento-tall-left { grid-column: span 3; grid-row: span 2; }
        .bento-wide { grid-column: span 3; }
        .bento-reg-3 { grid-column: span 3; }
        .bento-head { font-size: 0.72rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: var(--blue); margin-bottom: 0.875rem; }
        .bento h3 { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.025em; line-height: 1.2; margin-bottom: 0.625rem; color: var(--text); }
        .bento p { font-size: 0.9375rem; color: var(--muted); line-height: 1.55; max-width: 440px; }
        .bento-visual { margin-top: auto; padding-top: 1.5rem; }
        .bento-dark { background: var(--blue-deep); color: #fff; }
        .bento-dark .bento-head { color: #7dd3fc; }
        .bento-dark h3 { color: #fff; }
        .bento-dark p { color: rgba(255,255,255,0.7); }
        .bento-blue { background: linear-gradient(135deg, #eef4ff 0%, #e0e7ff 100%); }
        .bento-green { background: linear-gradient(135deg, #ecfdf5 0%, #d1fae5 100%); }

        /* viz-arb */
        .viz-arb { background: #fff; border-radius: 10px; overflow: hidden; border: 1px solid var(--border); box-shadow: 0 6px 18px rgba(50,50,93,0.08); }
        .viz-arb-head { background: #f9fafc; padding: 0.5rem 0.75rem; font-size: 0.68rem; font-weight: 600; color: var(--dim); border-bottom: 1px solid var(--border); text-transform: uppercase; letter-spacing: 0.05em; display: flex; justify-content: space-between; }
        .viz-arb-row { display: flex; align-items: center; gap: 0.5rem; padding: 0.55rem 0.75rem; border-bottom: 1px solid var(--border-sub); border-left: 3px solid var(--emerald); font-size: 0.78rem; }
        .viz-arb-row:last-child { border-bottom: none; }
        .viz-pill { font-family: var(--mono); font-size: 0.72rem; font-weight: 700; color: #fff; background: var(--emerald); padding: 0.15rem 0.4rem; border-radius: 4px; flex-shrink: 0; }
        .viz-ev-text { flex: 1; font-weight: 600; color: var(--text); font-size: 0.8125rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
        .viz-logo-pair { display: flex; gap: 0.25rem; flex-shrink: 0; }
        .viz-logo-pair img { width: 16px; height: 16px; border-radius: 3px; }

        /* viz-ev-grid */
        .viz-ev-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
        .ev-card { background: rgba(255,255,255,0.6); border-radius: 10px; padding: 0.875rem; backdrop-filter: blur(6px); border: 1px solid rgba(37,99,235,0.08); }
        .ev-card-lbl { font-size: 0.68rem; font-weight: 600; color: var(--dim); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.4rem; }
        .ev-card-row { display: flex; justify-content: space-between; align-items: center; font-size: 0.875rem; padding: 0.2rem 0; font-weight: 500; }
        .ev-num { font-family: var(--mono); font-weight: 700; color: var(--blue); }
        .ev-green { color: var(--emerald) !important; }

        /* viz-agent */
        .viz-agent { background: #0a2540; border-radius: 10px; padding: 1rem; color: #fff; font-family: var(--mono); font-size: 0.72rem; line-height: 1.55; }
        .viz-agent .k { color: #93c5fd; }
        .viz-agent .s { color: #86efac; }
        .viz-agent .c { color: #6b7280; }
        .viz-agent-blink::after { content: '▌'; color: #86efac; animation: blink 1s step-end infinite; }
        @keyframes blink { 0%,50%{opacity:1} 51%,100%{opacity:0} }

        /* ── Personas ── */
        .personas { padding: 6rem 2rem; background: var(--bg); font-family: var(--font); }
        .personas-inner { max-width: 1280px; margin: 0 auto; }
        .persona-grid { display: grid; grid-template-columns: repeat(3,1fr); gap: 1.25rem; }
        .persona { background: var(--bg-gray); border-radius: 16px; padding: 2rem; display: flex; flex-direction: column; gap: 1rem; transition: transform 0.25s, box-shadow 0.25s; text-decoration: none; color: inherit; }
        .persona:hover { transform: translateY(-4px); box-shadow: 0 24px 48px -20px rgba(50,50,93,0.15); }
        .persona-icon { width: 44px; height: 44px; background: #fff; border-radius: 10px; display: flex; align-items: center; justify-content: center; box-shadow: 0 2px 6px rgba(50,50,93,0.06); }
        .persona-icon svg { width: 22px; height: 22px; color: var(--blue); }
        .persona h4 { font-size: 1.25rem; font-weight: 700; letter-spacing: -0.02em; }
        .persona p { font-size: 0.9375rem; color: var(--muted); line-height: 1.55; }
        .persona-cta { font-size: 0.875rem; font-weight: 600; color: var(--blue); display: flex; align-items: center; gap: 0.3rem; margin-top: auto; }
        .persona-cta::after { content: '→'; transition: transform 0.2s; }
        .persona:hover .persona-cta::after { transform: translateX(3px); }

        /* ── Stats bar ── */
        .stats-bar { background: var(--bg-deep); color: #fff; padding: 5rem 2rem; position: relative; overflow: hidden; font-family: var(--font); }
        .stats-bar::before { content: ''; position: absolute; inset: 0; background: radial-gradient(ellipse at top right, rgba(37,99,235,0.25) 0%, transparent 60%); pointer-events: none; }
        .stats-inner { max-width: 1280px; margin: 0 auto; position: relative; }
        .stats-title { font-size: clamp(1.75rem, 3vw, 2.5rem); font-weight: 700; letter-spacing: -0.025em; line-height: 1.15; margin-bottom: 3rem; max-width: 600px; }
        .stats-title em { font-style: italic; background: linear-gradient(135deg, #93c5fd 0%, #86efac 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
        .stats-grid { display: grid; grid-template-columns: repeat(4,1fr); gap: 3rem; }
        .stat-item { border-top: 1px solid rgba(255,255,255,0.18); padding-top: 1.25rem; }
        .stat-n { font-size: 3.5rem; font-weight: 700; line-height: 1; letter-spacing: -0.035em; margin-bottom: 0.5rem; background: linear-gradient(135deg, #93c5fd 0%, #86efac 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
        .stat-l { font-size: 0.9375rem; color: rgba(255,255,255,0.7); line-height: 1.45; }

        /* ── Pricing ── */
        .pricing-section { padding: 6rem 2rem; background: var(--bg-gray); font-family: var(--font); }
        .pricing-inner { max-width: 1200px; margin: 0 auto; }
        .pricing-grid { display: grid; grid-template-columns: repeat(3,1fr); gap: 1.25rem; margin-top: 3rem; }
        .pc { background: #fff; border: 1px solid var(--border); border-radius: 16px; padding: 2rem; display: flex; flex-direction: column; gap: 1rem; transition: transform 0.2s, box-shadow 0.2s; }
        .pc:hover { transform: translateY(-3px); box-shadow: 0 16px 32px -12px rgba(50,50,93,0.1); }
        .pc.featured { background: var(--blue-deep); color: #fff; border-color: var(--blue-deep); }
        .pc.coming { background: linear-gradient(135deg, #fefce8 0%, #fef3c7 100%); border-color: #fde68a; }
        .pc-tier { font-size: 0.72rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: var(--dim); }
        .pc.featured .pc-tier { color: #93c5fd; }
        .pc.coming .pc-tier { color: #b45309; }
        .pc-name { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }
        .pc-price { display: flex; align-items: baseline; gap: 0.25rem; }
        .pc-num { font-size: 2.625rem; font-weight: 700; letter-spacing: -0.03em; line-height: 1; }
        .pc-per { font-size: 0.9375rem; color: var(--muted); }
        .pc.featured .pc-per { color: rgba(255,255,255,0.6); }
        .pc-coming-lbl { font-size: 1.25rem; font-weight: 700; color: #b45309; }
        .pc-desc { font-size: 0.9375rem; color: var(--muted); line-height: 1.5; }
        .pc.featured .pc-desc { color: rgba(255,255,255,0.7); }
        .pc.coming .pc-desc { color: #92400e; }
        .pc-feats { list-style: none; display: flex; flex-direction: column; gap: 0.6rem; flex: 1; margin-top: 0.25rem; padding: 0; }
        .pc-feats li { display: flex; gap: 0.6rem; font-size: 0.9375rem; color: var(--text-soft); line-height: 1.4; align-items: flex-start; }
        .pc.featured .pc-feats li { color: rgba(255,255,255,0.85); }
        .pc.coming .pc-feats li { color: #78350f; }
        .ck { width: 16px; height: 16px; margin-top: 3px; flex-shrink: 0; }
        .ck path { stroke: var(--emerald); stroke-width: 2.5; fill: none; stroke-linecap: round; stroke-linejoin: round; }
        .pc.featured .ck path { stroke: #86efac; }
        .pc.coming .ck path { stroke: #b45309; }
        .pc-btn { display: flex; align-items: center; justify-content: center; gap: 0.35rem; padding: 0.75rem 1.25rem; border-radius: 20px; font-size: 0.9375rem; font-weight: 600; text-decoration: none; margin-top: auto; transition: all 0.15s; cursor: pointer; border: none; font-family: var(--font); }
        .btn-deep { background: var(--blue-deep); color: #fff; }
        .btn-deep:hover { background: #173357; }
        .btn-white { background: #fff; color: var(--blue-deep); }
        .btn-white:hover { background: #f0f4ff; }
        .btn-amber { background: #b45309; color: #fff; opacity: 0.85; cursor: default; }

        /* ── CTA band ── */
        .cta-band { padding: 5rem 2rem; text-align: center; background: var(--bg); font-family: var(--font); }
        .cta-band h2 { font-size: clamp(2rem, 4vw, 3rem); font-weight: 700; letter-spacing: -0.03em; line-height: 1.1; margin-bottom: 1rem; color: var(--text); }
        .cta-band h2 em { font-style: italic; background: linear-gradient(135deg, #2563eb 0%, #367c53 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
        .cta-band p { font-size: 1.125rem; color: var(--muted); margin-bottom: 2rem; }
        .cta-band .hero-ctas { justify-content: center; }

        /* ── Footer ── */
        footer { background: #f6f9fc; color: var(--text-soft); padding: 4rem 2rem 2rem; border-top: 1px solid var(--border); font-family: var(--font); }
        .footer-top { max-width: 1280px; margin: 0 auto; display: grid; grid-template-columns: 1.4fr repeat(4,1fr); gap: 3rem; padding-bottom: 3rem; }
        .footer-brand img { height: 40px; margin-bottom: 1rem; display: block; }
        .footer-tagline { font-size: 0.875rem; color: var(--muted); line-height: 1.55; max-width: 280px; margin-bottom: 1.5rem; }
        .footer-socials { display: flex; gap: 0.75rem; }
        .footer-socials a { width: 34px; height: 34px; background: #fff; border: 1px solid var(--border); border-radius: 50%; display: flex; align-items: center; justify-content: center; color: var(--muted); text-decoration: none; transition: all 0.15s; }
        .footer-socials a:hover { color: var(--blue); border-color: var(--blue); }
        .footer-socials svg { width: 16px; height: 16px; }
        .footer-col h5 { font-size: 0.75rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: var(--text); margin-bottom: 1.25rem; }
        .footer-col a { display: block; font-size: 0.875rem; color: var(--muted); text-decoration: none; padding: 0.3rem 0; transition: color 0.15s; }
        .footer-col a:hover { color: var(--blue); }
        .footer-col a.dim { opacity: 0.55; }
        .footer-bottom { max-width: 1280px; margin: 0 auto; border-top: 1px solid var(--border); padding-top: 1.75rem; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 1rem; }
        .footer-copy { font-size: 0.8125rem; color: var(--dim); }
        .footer-legal { display: flex; gap: 1.5rem; }
        .footer-legal a { font-size: 0.8125rem; color: var(--dim); text-decoration: none; transition: color 0.15s; }
        .footer-legal a:hover { color: var(--blue); }
        .footer-disclaimer { max-width: 1280px; margin: 2rem auto 0; padding-top: 1.5rem; border-top: 1px solid var(--border); font-size: 0.75rem; color: var(--dim); line-height: 1.55; }

        /* ── Scroll reveal ── */
        .ai { opacity: 0; transform: translateY(20px); transition: opacity 0.55s ease, transform 0.55s ease; }
        .ai.v { opacity: 1; transform: translateY(0); }
        .d1 { transition-delay: 0.1s; }
        .d2 { transition-delay: 0.2s; }
        .d3 { transition-delay: 0.3s; }

        /* ── Responsive ── */
        @media (max-width: 1024px) {
          .bento-grid { grid-template-columns: repeat(2,1fr); }
          .bento-tall-left, .bento-wide, .bento-reg-3 { grid-column: span 2; grid-row: auto; }
          .persona-grid { grid-template-columns: 1fr; }
          .stats-grid { grid-template-columns: repeat(2,1fr); }
          .pricing-grid { grid-template-columns: 1fr; max-width: 400px; margin: 3rem auto 0; }
          .footer-top { grid-template-columns: 1fr 1fr; }
          .dash-body { grid-template-columns: 1fr; }
          .dash-sidebar { display: none; }
          .dash-stats { grid-template-columns: repeat(2,1fr); }
        }
        @media (max-width: 700px) {
          .nav-links { display: none; }
          .hero-section { padding: 2.5rem 1.25rem 3rem; }
          h1.hero-title { font-size: 2.75rem; }
          .section-title { font-size: 2rem; }
          .footer-top { grid-template-columns: 1fr; gap: 2rem; }
          .bento-grid { grid-template-columns: 1fr; }
          .bento-tall-left, .bento-wide, .bento-reg-3 { grid-column: auto; }
          .dash-thead, .dash-trow { grid-template-columns: 50px 1fr 80px; }
          .dash-thead span:nth-child(n+4), .dash-trow > :nth-child(n+4) { display: none; }
          .stats-grid { grid-template-columns: repeat(2,1fr); gap: 1.5rem; }
        }
      `}</style>
    </div>
  );
}
