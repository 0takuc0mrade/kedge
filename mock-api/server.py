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
import random
import time
from datetime import datetime, timedelta, timezone
from enum import Enum
from typing import Optional

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
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
        timestamp=int(now.timestamp()),
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
    print("   http://localhost:8089")
    print("─" * 45)
    
    uvicorn.run(app, host="0.0.0.0", port=8089)
