"use client";

import { FormEvent, useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  Check,
  CheckCircle2,
  Clock3,
  FileSignature,
  LoaderCircle,
  LockKeyhole,
  Radio,
  ShieldCheck,
  Ship,
  Wallet,
  X,
} from "lucide-react";
import { useWallet } from "./WalletProvider";

export const COVERAGE_STORAGE_KEY = "kedge-demo-coverage";

export type CoverageMandate = {
  policyId: string;
  trackingId: string;
  origin: string;
  destination: string;
  insuredValue: number;
  claimant: string;
  activatedAt: string;
  expiresAt: string;
  signature: string;
};

type ActivationStep = "details" | "review" | "signing" | "active";

function shortAddress(address: string) {
  return `${address.slice(0, 7)}...${address.slice(-5)}`;
}

function toHex(bytes: ArrayBuffer) {
  return Array.from(new Uint8Array(bytes))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

async function policyIdFor(value: string) {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(value),
  );
  return `0x${toHex(digest)}`;
}

export default function CoverageActivation({
  open,
  onClose,
  onActivated,
  existingMandate,
}: {
  open: boolean;
  onClose: () => void;
  onActivated: (mandate: CoverageMandate) => void;
  existingMandate: CoverageMandate | null;
}) {
  const { address, connecting, connect, signMessage } = useWallet();
  const [step, setStep] = useState<ActivationStep>("details");
  const [trackingId, setTrackingId] = useState("KDG-2026-0047");
  const [origin, setOrigin] = useState("Shanghai, CN");
  const [destination, setDestination] = useState("Rotterdam, NL");
  const [insuredValue, setInsuredValue] = useState("100000");
  const [error, setError] = useState("");
  const [mandate, setMandate] = useState<CoverageMandate | null>(null);

  const numericInsuredValue = Number(insuredValue) || 0;
  const premiumQuote = useMemo(
    () => Math.round(numericInsuredValue * 0.0125),
    [numericInsuredValue],
  );

  useEffect(() => {
    if (!open || !existingMandate) return;
    setMandate(existingMandate);
    setStep("active");
  }, [existingMandate, open]);

  if (!open) return null;

  const continueToReview = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    if (!trackingId.trim() || !origin.trim() || !destination.trim()) {
      setError("Complete the shipment details first.");
      return;
    }
    if (numericInsuredValue < 1_000) {
      setError("Insured value must be at least 1,000 mUSDT.");
      return;
    }
    if (!address) {
      try {
        await connect();
      } catch {
        setError("Connect the claimant wallet to continue.");
        return;
      }
    }
    setStep("review");
  };

  const activateCoverage = async () => {
    setStep("signing");
    setError("");
    try {
      const claimant = address || (await connect());
      const activatedAt = new Date();
      const expiresAt = new Date(
        activatedAt.getTime() + 30 * 24 * 60 * 60 * 1000,
      );
      const canonicalMandate = [
        "KEDGE COVERAGE MANDATE V1",
        `Tracking ID: ${trackingId.trim()}`,
        `Route: ${origin.trim()} -> ${destination.trim()}`,
        `Insured value: ${numericInsuredValue} mUSDT`,
        "Trigger: critical delay or loss at 48+ hours",
        "Payout tiers: 25% / 50% / 75% / 100%",
        `Claimant: ${claimant}`,
        `Chain ID: 5003`,
        `Valid from: ${activatedAt.toISOString()}`,
        `Valid until: ${expiresAt.toISOString()}`,
        "Intent: authorize Kedge to monitor and autonomously settle qualifying claims.",
      ].join("\n");
      const signature = await signMessage(canonicalMandate);
      const policyId = await policyIdFor(`${canonicalMandate}\n${signature}`);
      const nextMandate: CoverageMandate = {
        policyId,
        trackingId: trackingId.trim(),
        origin: origin.trim(),
        destination: destination.trim(),
        insuredValue: numericInsuredValue,
        claimant,
        activatedAt: activatedAt.toISOString(),
        expiresAt: expiresAt.toISOString(),
        signature,
      };

      localStorage.setItem(COVERAGE_STORAGE_KEY, JSON.stringify(nextMandate));
      setMandate(nextMandate);
      setStep("active");
      onActivated(nextMandate);
    } catch {
      setError("The activation signature was not completed. Please retry.");
      setStep("review");
    }
  };

  const resetAndClose = () => {
    setError("");
    if (step === "active" && !existingMandate) setStep("details");
    onClose();
  };

  return (
    <div className="activation-backdrop" role="presentation">
      <section
        className="activation-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="activation-title"
      >
        <div className="activation-header">
          <div>
            <span className="eyebrow">
              <ShieldCheck size={14} /> Coverage onboarding
            </span>
            <h2 id="activation-title">
              {step === "active"
                ? "Kedge is watching."
                : "Activate once. Settle autonomously."}
            </h2>
          </div>
          <button
            className="activation-close"
            type="button"
            onClick={resetAndClose}
            aria-label="Close coverage activation"
          >
            <X size={18} />
          </button>
        </div>

        <div className="activation-progress" aria-label="Activation progress">
          <span className="progress-complete">
            <Check size={13} /> Shipment
          </span>
          <span
            className={
              step === "review" || step === "signing" || step === "active"
                ? "progress-complete"
                : ""
            }
          >
            <Check size={13} /> Terms
          </span>
          <span className={step === "active" ? "progress-complete" : ""}>
            <Check size={13} /> Signature
          </span>
          <span className={step === "active" ? "progress-complete" : ""}>
            <Radio size={13} /> Monitoring
          </span>
        </div>

        {step === "details" && (
          <form className="activation-form" onSubmit={continueToReview}>
            <div className="activation-note">
              <LockKeyhole size={17} />
              <p>
                Your wallet signs one coverage mandate. It never approves or
                submits a future claim.
              </p>
            </div>

            <label className="field-wide">
              <span>Shipment tracking ID</span>
              <input
                value={trackingId}
                onChange={(event) => setTrackingId(event.target.value)}
                placeholder="KDG-2026-0047"
              />
            </label>
            <label>
              <span>Origin</span>
              <input
                value={origin}
                onChange={(event) => setOrigin(event.target.value)}
              />
            </label>
            <label>
              <span>Destination</span>
              <input
                value={destination}
                onChange={(event) => setDestination(event.target.value)}
              />
            </label>
            <label className="field-wide">
              <span>Insured cargo value / mUSDT</span>
              <input
                type="number"
                min="1000"
                step="1000"
                value={insuredValue}
                onChange={(event) => setInsuredValue(event.target.value)}
              />
            </label>

            <div className="claimant-row field-wide">
              <Wallet size={17} />
              <span>
                <small>Claimant wallet</small>
                {address ? shortAddress(address) : "Connect to assign claimant"}
              </span>
              <strong>{address ? "Connected" : "Required"}</strong>
            </div>

            {error && <p className="activation-error">{error}</p>}

            <div className="activation-actions field-wide">
              <span>
                Demo mandate / no premium is charged in this build
              </span>
              <button className="button button-primary" type="submit">
                {connecting ? (
                  <LoaderCircle className="wallet-spinner" size={16} />
                ) : null}
                Review coverage <ArrowRight size={16} />
              </button>
            </div>
          </form>
        )}

        {(step === "review" || step === "signing") && (
          <div className="activation-review">
            <div className="review-route">
              <div>
                <small>Origin</small>
                <strong>{origin}</strong>
              </div>
              <span>
                <Ship size={19} />
              </span>
              <div>
                <small>Destination</small>
                <strong>{destination}</strong>
              </div>
            </div>

            <div className="review-terms">
              <div>
                <span>Insured value</span>
                <strong>${numericInsuredValue.toLocaleString()}</strong>
              </div>
              <div>
                <span>Demo premium quote</span>
                <strong>${premiumQuote.toLocaleString()}</strong>
              </div>
              <div>
                <span>Monitoring period</span>
                <strong>30 days</strong>
              </div>
            </div>

            <div className="tier-table">
              <div>
                <span>48–71h</span>
                <strong>25%</strong>
              </div>
              <div>
                <span>72–119h</span>
                <strong>50%</strong>
              </div>
              <div>
                <span>120–239h</span>
                <strong>75%</strong>
              </div>
              <div>
                <span>240h+ / lost</span>
                <strong>100%</strong>
              </div>
            </div>

            <div className="signature-explainer">
              <FileSignature size={20} />
              <div>
                <strong>One signature starts the mandate</strong>
                <p>
                  After activation, Kedge monitors, proves, and settles without
                  another claimant transaction.
                </p>
              </div>
            </div>

            {error && <p className="activation-error">{error}</p>}

            <div className="activation-actions">
              <button
                className="button button-ghost"
                type="button"
                onClick={() => setStep("details")}
                disabled={step === "signing"}
              >
                Edit details
              </button>
              <button
                className="button button-primary"
                type="button"
                onClick={() => void activateCoverage()}
                disabled={step === "signing"}
              >
                {step === "signing" ? (
                  <>
                    <LoaderCircle className="wallet-spinner" size={16} />
                    Awaiting signature
                  </>
                ) : (
                  <>
                    Sign & activate <FileSignature size={16} />
                  </>
                )}
              </button>
            </div>
          </div>
        )}

        {step === "active" && mandate && (
          <div className="activation-success">
            <span className="success-icon">
              <CheckCircle2 size={34} />
            </span>
            <span className="eyebrow">Coverage mandate active</span>
            <h3>{mandate.trackingId}</h3>
            <p>
              The one-time claimant step is complete. Kedge now owns the next
              action whenever a signed logistics event breaches the policy.
            </p>
            <div className="success-policy">
              <span>
                <small>Policy reference</small>
                <code>
                  {mandate.policyId.slice(0, 12)}...
                  {mandate.policyId.slice(-8)}
                </code>
              </span>
              <span>
                <small>Monitoring window</small>
                <strong>
                  <Clock3 size={14} /> 30 days
                </strong>
              </span>
            </div>
            <div className="demo-boundary">
              This signed mandate is stored locally for the hackathon demo. A
              production release will anchor it in a PolicyRegistry and collect
              the quoted premium on-chain.
            </div>
            <button
              className="button button-primary"
              type="button"
              onClick={resetAndClose}
            >
              Open monitoring desk <Radio size={16} />
            </button>
          </div>
        )}
      </section>
    </div>
  );
}
