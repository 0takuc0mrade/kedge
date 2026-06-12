"""
Kedge Mock Freight API Server

Simulates a maritime shipping tracking API for the Kedge
autonomous claim adjuster. Returns randomized shipment
telemetry data that cycles through different delay scenarios.

Usage:
    pip install fastapi uvicorn
    python server.py

Endpoints:
    GET /api/v1/shipment/latest   → Latest shipment status
    GET /api/v1/shipment/{id}     → Specific shipment by tracking ID
    POST /api/v1/shipment/config  → Configure delay scenario
    GET /health                   → Health check
"""

import json
import hashlib
import os
import random
import time
from datetime import datetime, timedelta, timezone
from enum import Enum
from typing import Optional

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from pydantic import BaseModel

app = FastAPI(
    title="Kedge Mock Freight API",
    description="Simulated maritime shipping telemetry for parametric insurance testing",
    version="0.1.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


# ─── Data Models ─────────────────────────────────────────────

class ShipmentStatus(str, Enum):
    ON_TIME = "on_time"
    DELAYED = "delayed"
    CRITICAL_DELAY = "critical_delay"
    LOST = "lost"


class ShipmentData(BaseModel):
    tracking_id: str
    carrier: str
    origin: str
    destination: str
    status: ShipmentStatus
    delay_hours: int
    estimated_delivery: str
    actual_delivery: Optional[str] = None
    insured_value: int
    timestamp: int
    policy_id: str
    claimant: str
    nonce: int
    issued_at: int
    expires_at: int
    chain_id: int
    oracle_public_key: str
    oracle_signature: str


class ScenarioConfig(BaseModel):
    """Configure the mock API to return a specific scenario."""
    scenario: str  # "on_time", "minor_delay", "critical", "total_loss", "random"
    delay_hours: Optional[int] = None
    insured_value: Optional[int] = None


# ─── Scenario Definitions ───────────────────────────────────

CARRIERS = ["Maersk Global", "MSC Shipping", "CMA CGM", "COSCO Lines", "Hapag-Lloyd"]

ROUTES = [
    ("Shanghai, CN", "Rotterdam, NL"),
    ("Shenzhen, CN", "Los Angeles, US"),
    ("Singapore, SG", "Hamburg, DE"),
    ("Busan, KR", "Long Beach, US"),
    ("Hong Kong, HK", "Felixstowe, UK"),
]

SCENARIOS = {
    "on_time": {"status": ShipmentStatus.ON_TIME, "delay_range": (0, 12)},
    "minor_delay": {"status": ShipmentStatus.DELAYED, "delay_range": (12, 47)},
    "critical": {"status": ShipmentStatus.CRITICAL_DELAY, "delay_range": (48, 200)},
    "total_loss": {"status": ShipmentStatus.LOST, "delay_range": (240, 500)},
}


# ─── Global State ────────────────────────────────────────────

class APIState:
    """Mutable state for the mock API."""
    def __init__(self):
        self.current_scenario: str = "random"
        self.fixed_delay: Optional[int] = None
        self.insured_value: int = 50_000
        self.call_count: int = 0

state = APIState()

environment = os.getenv("KEDGE_ENV", "development").lower()
is_production = environment in {"production", "testnet"}


def load_hex_env(name: str, expected_bytes: int, default: Optional[bytes] = None) -> bytes:
    value = os.getenv(name)
    if not value and default is None:
        raise RuntimeError(f"{name} is required when KEDGE_ENV={environment}")
    decoded = bytes.fromhex(value.removeprefix("0x")) if value else default
    if len(decoded) != expected_bytes:
        raise RuntimeError(f"{name} must contain exactly {expected_bytes} bytes")
    return decoded


oracle_private_key_hex = os.getenv("ORACLE_PRIVATE_KEY_HEX")
if oracle_private_key_hex:
    oracle_private_key = Ed25519PrivateKey.from_private_bytes(
        load_hex_env("ORACLE_PRIVATE_KEY_HEX", 32, b"")
    )
    ephemeral_oracle = False
elif is_production:
    raise RuntimeError(
        f"ORACLE_PRIVATE_KEY_HEX is required when KEDGE_ENV={environment}"
    )
else:
    oracle_private_key = Ed25519PrivateKey.generate()
    ephemeral_oracle = True

oracle_public_key = oracle_private_key.public_key().public_bytes(
    encoding=serialization.Encoding.Raw,
    format=serialization.PublicFormat.Raw,
)
policy_id = load_hex_env(
    "KEDGE_POLICY_ID",
    32,
    None if is_production else hashlib.sha256(b"KEDGE-DEMO-POLICY-001").digest(),
)
claimant = load_hex_env(
    "KEDGE_CLAIMANT_ADDRESS",
    20,
    None if is_production else bytes.fromhex("42" * 20),
)
chain_id = int(os.getenv("CHAIN_ID", "5003"))
oracle_ttl_secs = int(os.getenv("ORACLE_TTL_SECS", "300"))


def status_code(status: ShipmentStatus) -> int:
    return {
        ShipmentStatus.ON_TIME: 0,
        ShipmentStatus.DELAYED: 1,
        ShipmentStatus.CRITICAL_DELAY: 2,
        ShipmentStatus.LOST: 3,
    }[status]


def oracle_signing_bytes(
    tracking_id: str,
    status: ShipmentStatus,
    delay_hours: int,
    insured_value: int,
    event_timestamp: int,
    nonce: int,
    issued_at: int,
    expires_at: int,
) -> bytes:
    tracking_bytes = tracking_id.encode("utf-8")
    return b"".join(
        [
            b"KEDGE_ORACLE_V1",
            len(tracking_bytes).to_bytes(4, "big"),
            tracking_bytes,
            policy_id,
            claimant,
            status_code(status).to_bytes(1, "big"),
            delay_hours.to_bytes(8, "big"),
            insured_value.to_bytes(8, "big"),
            event_timestamp.to_bytes(8, "big"),
            nonce.to_bytes(8, "big"),
            issued_at.to_bytes(8, "big"),
            expires_at.to_bytes(8, "big"),
            chain_id.to_bytes(8, "big"),
        ]
    )


# ─── Shipment Generator ────────────────────────────────────

def generate_shipment(tracking_id: Optional[str] = None) -> ShipmentData:
    """Generate a shipment data point based on the current scenario."""
    state.call_count += 1

    # Pick scenario
    if state.current_scenario == "random":
        # Weighted random: bias toward critical delays for demo impact
        scenario_name = random.choices(
            ["on_time", "minor_delay", "critical", "total_loss"],
            weights=[20, 20, 45, 15],
            k=1,
        )[0]
    else:
        scenario_name = state.current_scenario

    scenario = SCENARIOS[scenario_name]
    
    # Generate delay
    if state.fixed_delay is not None:
        delay_hours = state.fixed_delay
    else:
        delay_hours = random.randint(*scenario["delay_range"])

    # Pick route and carrier
    origin, destination = random.choice(ROUTES)
    carrier = random.choice(CARRIERS)

    # Generate timestamps
    now = datetime.now(timezone.utc)
    estimated = now - timedelta(hours=delay_hours)
    
    tid = tracking_id or f"KDG-2026-{state.call_count:04d}"
    event_timestamp = int(now.timestamp())
    issued_at = event_timestamp
    expires_at = issued_at + oracle_ttl_secs
    signing_bytes = oracle_signing_bytes(
        tracking_id=tid,
        status=scenario["status"],
        delay_hours=delay_hours,
        insured_value=state.insured_value,
        event_timestamp=event_timestamp,
        nonce=state.call_count,
        issued_at=issued_at,
        expires_at=expires_at,
    )
    signature = oracle_private_key.sign(signing_bytes)

    return ShipmentData(
        tracking_id=tid,
        carrier=carrier,
        origin=origin,
        destination=destination,
        status=scenario["status"],
        delay_hours=delay_hours,
        estimated_delivery=estimated.isoformat(),
        actual_delivery=None if delay_hours > 0 else now.isoformat(),
        insured_value=state.insured_value,
        timestamp=event_timestamp,
        policy_id=f"0x{policy_id.hex()}",
        claimant=f"0x{claimant.hex()}",
        nonce=state.call_count,
        issued_at=issued_at,
        expires_at=expires_at,
        chain_id=chain_id,
        oracle_public_key=f"0x{oracle_public_key.hex()}",
        oracle_signature=f"0x{signature.hex()}",
    )


# ─── Endpoints ──────────────────────────────────────────────

@app.get("/health")
def health_check():
    """Health check endpoint."""
    return {
        "status": "ok",
        "service": "kedge-mock-freight-api",
        "scenario": state.current_scenario,
        "calls_served": state.call_count,
        "oracle_public_key": f"0x{oracle_public_key.hex()}",
        "oracle_key_hash": f"0x{hashlib.sha256(oracle_public_key).hexdigest()}",
        "ephemeral_oracle": ephemeral_oracle,
    }


@app.get("/api/v1/shipment/latest", response_model=ShipmentData)
def get_latest_shipment():
    """
    Get the latest shipment status.
    
    This is the primary endpoint that the Kedge agent polls
    on each evaluation cycle.
    """
    return generate_shipment()


@app.get("/api/v1/shipment/{tracking_id}", response_model=ShipmentData)
def get_shipment_by_id(tracking_id: str):
    """Get shipment status by tracking ID."""
    return generate_shipment(tracking_id=tracking_id)


@app.post("/api/v1/shipment/config")
def configure_scenario(config: ScenarioConfig):
    """
    Configure the mock API to return a specific scenario.
    
    Useful for demo workflows where you want deterministic behavior:
    - "on_time": Shipment arrives on schedule
    - "minor_delay": Small delay, below claim threshold
    - "critical": Delay exceeds parametric threshold → triggers claim
    - "total_loss": Shipment lost → maximum payout
    - "random": Weighted random selection (default)
    """
    if config.scenario not in list(SCENARIOS.keys()) + ["random"]:
        raise HTTPException(
            status_code=400,
            detail=f"Unknown scenario '{config.scenario}'. Valid: {list(SCENARIOS.keys()) + ['random']}",
        )

    state.current_scenario = config.scenario
    state.fixed_delay = config.delay_hours
    if config.insured_value is not None:
        state.insured_value = config.insured_value

    return {
        "status": "configured",
        "scenario": state.current_scenario,
        "fixed_delay_hours": state.fixed_delay,
        "insured_value": state.insured_value,
    }


# ─── Entry Point ────────────────────────────────────────────

if __name__ == "__main__":
    import uvicorn
    
    print("🚢 Kedge Mock Freight API")
    print("   Simulating maritime shipping telemetry")
    print(f"   Oracle public key: 0x{oracle_public_key.hex()}")
    print(f"   Oracle key hash:   0x{hashlib.sha256(oracle_public_key).hexdigest()}")
    if ephemeral_oracle:
        print("   WARNING: using an ephemeral oracle key; set ORACLE_PRIVATE_KEY_HEX for stable deployments")
    print("   http://localhost:8089")
    print("─" * 45)
    
    uvicorn.run(app, host="0.0.0.0", port=8089)
