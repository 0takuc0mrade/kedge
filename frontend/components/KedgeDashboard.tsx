"use client";

import { useEffect, useRef, useState } from "react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import {
  Activity,
  ArrowDownRight,
  ArrowUpRight,
  BadgeCheck,
  Box,
  Check,
  ChevronRight,
  CircleDot,
  Clock3,
  Copy,
  ExternalLink,
  Fingerprint,
  Gauge,
  Globe2,
  LockKeyhole,
  Menu,
  Radio,
  Route,
  ServerCog,
  ShieldCheck,
  Ship,
  Sparkles,
  WalletCards,
  X,
  Zap,
} from "lucide-react";
import OceanField from "./OceanField";
import WalletButton from "./WalletButton";
import CoverageActivation, {
  COVERAGE_STORAGE_KEY,
  CoverageMandate,
} from "./CoverageActivation";

gsap.registerPlugin(ScrollTrigger);

const contracts = [
  {
    label: "Claim registry",
    address: "0xC5e3...2314b",
    full: "0xC5e3Ffa61D3B3e2c8d8F132917e595A19562314b",
  },
  {
    label: "Settlement vault",
    address: "0x1FC3...25dDA",
    full: "0x1FC353C48b00F5DaB51c736828fb08EcA7F25dDA",
  },
  {
    label: "Agent identity",
    address: "0x1A47...2fb03",
    full: "0x1A47e658ddD31C18ec801BE2128F7cF873d2fb03",
  },
];

const proofSteps = [
  {
    number: "01",
    title: "Signal ingested",
    detail: "Signed carrier and port telemetry enters the policy envelope.",
    metric: "12 sources",
    icon: Radio,
  },
  {
    number: "02",
    title: "Policy executed",
    detail: "The no_std evaluator resolves delay, coverage, and payout.",
    metric: "1.8 sec",
    icon: ServerCog,
  },
  {
    number: "03",
    title: "Proof sealed",
    detail: "RISC Zero attests the exact claim computation in Groth16.",
    metric: "256 bit",
    icon: Fingerprint,
  },
  {
    number: "04",
    title: "Funds cleared",
    detail: "Mantle verifies agent identity and releases the stablecoin.",
    metric: "final",
    icon: Zap,
  },
];

const activity = [
  {
    time: "22:41:08",
    route: "Shanghai / Rotterdam",
    claim: "KDG-0047",
    amount: "$50,000",
    status: "Settled",
  },
  {
    time: "22:36:52",
    route: "Lagos / Antwerp",
    claim: "KDG-0046",
    amount: "$18,400",
    status: "Verified",
  },
  {
    time: "22:29:14",
    route: "Busan / Long Beach",
    claim: "KDG-0045",
    amount: "$72,800",
    status: "Observed",
  },
];

function ExplorerLink({
  address,
  children,
}: {
  address: string;
  children: React.ReactNode;
}) {
  return (
    <a
      href={`https://explorer.sepolia.mantle.xyz/address/${address}`}
      target="_blank"
      rel="noreferrer"
    >
      {children}
    </a>
  );
}

export default function KedgeDashboard() {
  const rootRef = useRef<HTMLElement>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [copied, setCopied] = useState("");
  const [activationOpen, setActivationOpen] = useState(false);
  const [coverage, setCoverage] = useState<CoverageMandate | null>(null);

  useEffect(() => {
    if (new URLSearchParams(window.location.search).get("activate") === "1") {
      setActivationOpen(true);
    }

    const storedCoverage = localStorage.getItem(COVERAGE_STORAGE_KEY);
    if (storedCoverage) {
      try {
        setCoverage(JSON.parse(storedCoverage) as CoverageMandate);
      } catch {
        localStorage.removeItem(COVERAGE_STORAGE_KEY);
      }
    }

    const reducedMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (reducedMotion) return;

    const context = gsap.context(() => {
      const intro = gsap.timeline({ defaults: { ease: "power3.out" } });
      intro
        .from(".site-header", { y: -24, duration: 0.7 })
        .from(
          ".hero-kicker, .hero-title > span, .hero-copy, .hero-actions",
          {
            y: 34,
            duration: 0.8,
            stagger: 0.09,
          },
          "-=0.35",
        )
        .from(
          ".mission-stage",
          { scale: 0.96, duration: 1 },
          "-=0.85",
        )
        .from(
          ".claim-console",
          { x: 32, duration: 0.75 },
          "-=0.55",
        );

      gsap.utils.toArray<HTMLElement>("[data-reveal]").forEach((element) => {
        gsap.from(element, {
          y: 44,
          opacity: 0,
          duration: 0.85,
          ease: "power3.out",
          scrollTrigger: {
            trigger: element,
            start: "top 86%",
            once: true,
          },
        });
      });

      gsap.to(".radar-sweep", {
        rotate: 360,
        duration: 8,
        repeat: -1,
        ease: "none",
      });
    }, rootRef);

    return () => context.revert();
  }, []);

  const copyAddress = async (address: string) => {
    await navigator.clipboard.writeText(address);
    setCopied(address);
    window.setTimeout(() => setCopied(""), 1400);
  };

  return (
    <main ref={rootRef} className="site-shell">
      <header className="site-header">
        <a className="brand" href="#top" aria-label="Kedge home">
          <span className="brand-mark">
            <span />
            <span />
            <span />
          </span>
          <span className="brand-word">KEDGE</span>
          <span className="brand-tag">Autonomous adjuster</span>
        </a>

        <nav className={menuOpen ? "nav-links nav-links-open" : "nav-links"}>
          <a href="#desk" onClick={() => setMenuOpen(false)}>
            Live desk
          </a>
          <a href="#proof" onClick={() => setMenuOpen(false)}>
            Proof route
          </a>
          <a href="#network" onClick={() => setMenuOpen(false)}>
            Network
          </a>
          <button
            className={coverage ? "nav-coverage nav-coverage-active" : "nav-coverage"}
            type="button"
            onClick={() => {
              setMenuOpen(false);
              setActivationOpen(true);
            }}
          >
            {coverage ? "Coverage active" : "Activate coverage"}
          </button>
          <WalletButton />
          <a
            className="nav-cta"
            href="https://explorer.sepolia.mantle.xyz/address/0xC5e3Ffa61D3B3e2c8d8F132917e595A19562314b"
            target="_blank"
            rel="noreferrer"
          >
            View on Mantle <ArrowUpRight size={14} />
          </a>
        </nav>

        <button
          className="menu-button"
          type="button"
          onClick={() => setMenuOpen((open) => !open)}
          aria-label="Toggle navigation"
          aria-expanded={menuOpen}
        >
          {menuOpen ? <X size={20} /> : <Menu size={20} />}
        </button>
      </header>

      <section className="hero" id="top">
        <div className="hero-copy-column">
          <div className="hero-kicker">
            <span className="live-pulse" />
            Autonomous clearinghouse / Mantle Sepolia
          </div>
          <h1 className="hero-title">
            <span>Freight delayed.</span>
            <span className="title-accent">Claim cleared.</span>
          </h1>
          <p className="hero-copy">
            Kedge watches global cargo, executes the policy inside a zkVM, and
            settles verified claims before a human adjuster opens the file.
          </p>
          <div className="hero-actions">
            <button
              className="button button-primary"
              type="button"
              onClick={() => setActivationOpen(true)}
            >
              {coverage ? "View active coverage" : "Activate coverage"}
              {coverage ? <ShieldCheck size={17} /> : <ArrowDownRight size={17} />}
            </button>
            <a className="button button-ghost" href="#proof">
              Trace a proof <Route size={17} />
            </a>
          </div>
          {coverage && (
            <button
              className="coverage-monitoring"
              type="button"
              onClick={() => setActivationOpen(true)}
            >
              <span className="live-pulse" />
              <span>
                Monitoring <strong>{coverage.trackingId}</strong>
              </span>
              <ArrowUpRight size={14} />
            </button>
          )}
          <div className="hero-trust">
            <div>
              <ShieldCheck size={17} />
              <span>RISC Zero verified</span>
            </div>
            <div>
              <Fingerprint size={17} />
              <span>ERC-8004 identity</span>
            </div>
            <div>
              <Zap size={17} />
              <span>Instant settlement</span>
            </div>
          </div>
        </div>

        <div className="mission-stage" aria-label="Live Kedge claim route">
          <OceanField />
          <div className="stage-grid" />
          <div className="stage-label stage-label-left">
            <span>Origin / CN-SHA</span>
            <strong>Shanghai</strong>
          </div>
          <div className="stage-label stage-label-right">
            <span>Destination / NL-RTM</span>
            <strong>Rotterdam</strong>
          </div>

          <div className="radar">
            <div className="radar-ring radar-ring-one" />
            <div className="radar-ring radar-ring-two" />
            <div className="radar-sweep" />
            <div className="vessel">
              <Ship size={22} />
            </div>
          </div>

          <div className="delay-chip">
            <Clock3 size={15} />
            <span>
              Delay signal <strong>+96h</strong>
            </span>
          </div>

          <div className="proof-packet">
            <span className="packet-icon">
              <Fingerprint size={18} />
            </span>
            <span>
              <small>Proof packet</small>
              0x8e1f...c942
            </span>
            <BadgeCheck size={17} />
          </div>

          <div className="stage-readout">
            <span>47.3769 N</span>
            <span>08.5417 E</span>
            <span>SEA STATE 03</span>
          </div>
        </div>

        <aside className="claim-console">
          <div className="console-heading">
            <span>
              <Activity size={15} /> Active claim
            </span>
            <span className="console-id">KDG-2026-0047</span>
          </div>
          <div className="console-status">
            <span className="status-orbit">
              <span />
            </span>
            <div>
              <small>Decision</small>
              <strong>Payable</strong>
            </div>
            <span className="confidence">99.98%</span>
          </div>
          <div className="console-data">
            <div>
              <span>Policy</span>
              <strong>Critical delay ≥ 48h</strong>
            </div>
            <div>
              <span>Observed</span>
              <strong>96h 14m</strong>
            </div>
            <div>
              <span>Coverage</span>
              <strong>$50,000.00</strong>
            </div>
            <div>
              <span>Agent ID</span>
              <strong>KAI / 0001</strong>
            </div>
          </div>
          <div className="console-progress">
            <span>
              <Check size={13} /> Signal
            </span>
            <span>
              <Check size={13} /> Evaluate
            </span>
            <span>
              <Check size={13} /> Prove
            </span>
            <span className="progress-current">
              <CircleDot size={13} /> Settle
            </span>
          </div>
          <div className="console-footer">
            <span>Automated action</span>
            <strong>Release 50,000 mUSDT</strong>
          </div>
        </aside>
      </section>

      <section className="signal-strip" aria-label="Kedge network statistics">
        <div>
          <span className="signal-icon">
            <Globe2 size={18} />
          </span>
          <p>
            <strong>12</strong>
            <span>Live data feeds</span>
          </p>
        </div>
        <div>
          <span className="signal-icon">
            <Gauge size={18} />
          </span>
          <p>
            <strong>1.8s</strong>
            <span>Decision latency</span>
          </p>
        </div>
        <div>
          <span className="signal-icon">
            <LockKeyhole size={18} />
          </span>
          <p>
            <strong>100%</strong>
            <span>Verified execution</span>
          </p>
        </div>
        <div>
          <span className="signal-icon">
            <WalletCards size={18} />
          </span>
          <p>
            <strong>$100k</strong>
            <span>Vault liquidity</span>
          </p>
        </div>
      </section>

      <section className="desk-section section-wrap" id="desk">
        <div className="section-heading" data-reveal>
          <div>
            <span className="eyebrow">Live settlement desk</span>
            <h2>One claim. No black box.</h2>
          </div>
          <p>
            Every automated decision remains legible from raw logistics signal
            to final on-chain transfer.
          </p>
        </div>

        <div className="desk-grid">
          <article className="policy-card glass-card" data-reveal>
            <div className="card-topline">
              <span>Parametric cover</span>
              <span className="status-pill">Policy active</span>
            </div>
            <div className="policy-route">
              <div>
                <span>CN SHA</span>
                <strong>Shanghai</strong>
              </div>
              <div className="route-line">
                <span />
                <Ship size={19} />
                <span />
              </div>
              <div>
                <span>NL RTM</span>
                <strong>Rotterdam</strong>
              </div>
            </div>
            <div className="policy-condition">
              <small>Trigger condition</small>
              <div>
                <Clock3 size={21} />
                <span>
                  Critical delay reaches <strong>48 hours</strong>
                </span>
              </div>
            </div>
            <div className="policy-metrics">
              <div>
                <span>Insured value</span>
                <strong>$250,000</strong>
              </div>
              <div>
                <span>Payout rate</span>
                <strong>20%</strong>
              </div>
              <div>
                <span>Max payout</span>
                <strong>$50,000</strong>
              </div>
            </div>
          </article>

          <article className="decision-card glass-card" data-reveal>
            <div className="decision-orb">
              <svg viewBox="0 0 160 160" aria-hidden="true">
                <circle className="orb-track" cx="80" cy="80" r="66" />
                <circle className="orb-value" cx="80" cy="80" r="66" />
              </svg>
              <div>
                <Sparkles size={22} />
                <strong>PAY</strong>
                <span>Policy resolved</span>
              </div>
            </div>
            <div className="decision-copy">
              <span className="eyebrow">Autonomous verdict</span>
              <h3>Threshold breached by 24h 14m.</h3>
              <p>
                The policy terms, signed observation, and payout calculation
                were committed to the receipt journal.
              </p>
              <div className="journal-row">
                <span>Journal hash</span>
                <code>0x75b2...88af</code>
                <button
                  type="button"
                  aria-label="Copy journal hash"
                  onClick={() => copyAddress("0x75b2...88af")}
                >
                  {copied === "0x75b2...88af" ? (
                    <Check size={14} />
                  ) : (
                    <Copy size={14} />
                  )}
                </button>
              </div>
            </div>
          </article>

          <article className="settlement-card glass-card" data-reveal>
            <div className="card-topline">
              <span>Settlement order</span>
              <BadgeCheck size={18} />
            </div>
            <div className="settlement-amount">
              <span>mUSDT</span>
              <strong>50,000.00</strong>
              <small>Released from the Kedge vault</small>
            </div>
            <div className="settlement-flow">
              <div>
                <Box size={18} />
                <span>
                  <small>From</small>
                  Settlement vault
                </span>
              </div>
              <ChevronRight size={16} />
              <div>
                <WalletCards size={18} />
                <span>
                  <small>To</small>
                  0x3cE3...b192
                </span>
              </div>
            </div>
            <a
              className="transaction-link"
              href="https://explorer.sepolia.mantle.xyz/address/0x1FC353C48b00F5DaB51c736828fb08EcA7F25dDA"
              target="_blank"
              rel="noreferrer"
            >
              Inspect settlement vault <ExternalLink size={14} />
            </a>
          </article>
        </div>
      </section>

      <section className="proof-section section-wrap" id="proof">
        <div className="section-heading proof-heading" data-reveal>
          <div>
            <span className="eyebrow">The proof route</span>
            <h2>Autonomy, with receipts.</h2>
          </div>
          <p>
            Kedge does not ask the chain to trust its conclusion. It proves the
            exact program and inputs that produced it.
          </p>
        </div>

        <div className="proof-track">
          {proofSteps.map((step) => {
            const Icon = step.icon;
            return (
              <article className="proof-step" data-reveal key={step.number}>
                <div className="proof-step-top">
                  <span>{step.number}</span>
                  <Icon size={21} />
                </div>
                <h3>{step.title}</h3>
                <p>{step.detail}</p>
                <div className="proof-metric">
                  <span />
                  {step.metric}
                </div>
              </article>
            );
          })}
        </div>
      </section>

      <section className="network-section section-wrap" id="network">
        <div className="network-panel" data-reveal>
          <div className="network-intro">
            <span className="eyebrow">Deployed infrastructure</span>
            <h2>Alive on Mantle.</h2>
            <p>
              The Kedge identity, registry, and vault are deployed together on
              Mantle Sepolia. Each address opens directly in the explorer.
            </p>
            <div className="network-badge">
              <span className="live-pulse" />
              Chain ID 5003 / healthy
            </div>
          </div>

          <div className="contract-list">
            {contracts.map((contract) => (
              <div className="contract-row" key={contract.label}>
                <span>
                  <small>{contract.label}</small>
                  <code>{contract.address}</code>
                </span>
                <div>
                  <button
                    type="button"
                    aria-label={`Copy ${contract.label} address`}
                    onClick={() => copyAddress(contract.full)}
                  >
                    {copied === contract.full ? (
                      <Check size={16} />
                    ) : (
                      <Copy size={16} />
                    )}
                  </button>
                  <ExplorerLink address={contract.full}>
                    <ArrowUpRight size={17} />
                  </ExplorerLink>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="activity-panel" data-reveal>
          <div className="activity-heading">
            <div>
              <span className="live-pulse" />
              Recent agent activity
            </div>
            <span>UTC</span>
          </div>
          <div className="activity-table">
            <div className="activity-row activity-labels">
              <span>Time</span>
              <span>Route</span>
              <span>Claim</span>
              <span>Amount</span>
              <span>Status</span>
            </div>
            {activity.map((item) => (
              <div className="activity-row" key={item.claim}>
                <span>{item.time}</span>
                <span>{item.route}</span>
                <span>{item.claim}</span>
                <span>{item.amount}</span>
                <span className={`activity-status status-${item.status.toLowerCase()}`}>
                  {item.status}
                </span>
              </div>
            ))}
          </div>
        </div>
      </section>

      <footer className="site-footer">
        <div className="brand footer-brand">
          <span className="brand-mark">
            <span />
            <span />
            <span />
          </span>
          <span className="brand-word">KEDGE</span>
        </div>
        <p>Autonomous claim infrastructure for real-world trade.</p>
        <div>
          <a href="#top">Back to top</a>
          <a
            href="https://github.com/0takuc0mrade/kedge"
            target="_blank"
            rel="noreferrer"
          >
            GitHub <ArrowUpRight size={13} />
          </a>
        </div>
      </footer>

      <CoverageActivation
        open={activationOpen}
        onClose={() => setActivationOpen(false)}
        onActivated={setCoverage}
        existingMandate={coverage}
      />
    </main>
  );
}
