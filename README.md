# PLATO: Platform for Lifecycle, Assurance, Telemetry, and Orbit-aware Operations

<div align="center">

[![Rust Version](https://img.shields.io/badge/Rust-2021_Edition-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/API_Framework-Axum_0.7-blue.svg?style=flat-square)](https://github.com/tokio-rs/axum)
[![Next.js](https://img.shields.io/badge/Frontend-Next.js_14-black.svg?style=flat-square&logo=next.js)](https://nextjs.org/)
[![Database](https://img.shields.io/badge/Storage-PostgreSQL_16-336791.svg?style=flat-square&logo=postgresql)](https://www.postgresql.org/)
[![Event Bus](https://img.shields.io/badge/Event_Bus-NATS_JetStream-27AAE1.svg?style=flat-square&logo=nats.io)](https://nats.io/)
[![Orbital Engine](https://img.shields.io/badge/Propagator-SGP4_Ephemeris-4B0082.svg?style=flat-square)](#orbital-kinematics--pass-aware-scheduling-cratesorbit)
[![License](https://img.shields.io/badge/License-Apache_2.0_/_MIT-green.svg?style=flat-square)](#license)

**The Mission Control DevSecOps Control Plane for Satellite Mega-Constellations.**  
*Bridging the divide between Git commits on Earth and autonomous, cryptographically verified software execution in orbit.*

> 🚀 **Quick Guide:** Explore the live interactive operator & flight verification guide at **[https://plato-3afac.web.app/](https://plato-3afac.web.app/)**

[Quick Guide (Web App)](https://plato-3afac.web.app/) •
[Operator Verification Guide](../guide/GUIDE.md) •
[Executive Summary](#executive-summary--the-yc-thesis) •
[Objectives](#core-objectives-of-plato) •
[Competitive Landscape](#competitive-landscape--alternatives-analysis) •
[Architecture Whitepaper](#system-architecture-whitepaper) •
[Launch Guide](#exact-steps-to-launch-plato) •
[API Reference](#api-reference--endpoint-tour)

---

</div>

## Executive Summary, Platform Overview & Design Rationale

Modern commercial space is experiencing an exponential inflection point: satellite constellations are shifting from bespoke, single-spacecraft science missions to **mega-constellations comprising hundreds to thousands of networked orbital assets** (Starlink, Kuiper, Planet, Spire, defense mesh swarms). 

Yet, the operational tooling powering flight software updates remains fundamentally broken:

> **The Space Software Delivery Gap:**  
> Terrestrial software engineering practices (Git workflows, automated CI/CD, hermetic container builds, unit testing, SBOM verification) are completely severed from Mission Operations (ground stations, telemetry tracking, orbital passes, telecommanding).

Today, satellite operators bridge this divide through **manual handoffs, fragmented test logs, ad-hoc Python/Bash scripts, spreadsheets, emails, and unverified binary uploads during 5-to-15 minute line-of-sight ground passes**. This introduces catastrophic operational risks:
1. **Bricking Risk:** An unverified firmware update can permanently incapacitate a $50M+ spacecraft or leave it uncontrolled in Low Earth Orbit (LEO).
2. **Zero End-to-End Traceability:** Inquiries into *what exact Git commit or compiler flag is running on Satellite-42* require scouring months of dispersed Slack logs, Jira tickets, and raw RF packets.
3. **Constellation Bottleneck:** Ground control teams cannot manually coordinate phased, canary, or flight-safe rollouts across 500+ assets with heterogeneous orbital inclinations, dynamic thermal states, and intermittent contact windows.

### The PLATO Mission
**PLATO is a Space Software Delivery Platform acting as an immutable DevSecOps control plane.**  
It provides a reliable, traceable, cryptographically gated, and pass-aware mechanism for delivering approved software releases to commercial spacecraft and satellite fleets.

PLATO does **not** fly the satellite, dictate attitude control, or replace real-time flight operating systems (e.g., RTEMS, FreeRTOS, VxWorks, embedded Linux). Rather, **PLATO orchestrates the entire software release and delivery lifecycle around the spacecraft**, bridging the critical gap between Git commit hashes and Ground-Station-as-a-Service (GSaaS) RF transmitters.

---

## Core Objectives of PLATO

PLATO is engineered around five inviolable architectural tenets:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            PLATO CORE TENETS                                │
├─────────────────────────┬─────────────────────────┬─────────────────────────┤
│   SAFETY BEFORE SPEED   │  END-TO-END TRACEABILITY│    VENDOR NEUTRALITY    │
│ Zero unverified code    │ Immutable write-once    │ Decoupled from bus,     │
│ reaches space; strict   │ audit log binding Git   │ RTOS, payload, and GSaaS│
│ cryptographic gates.    │ commit to orbit state.  │ RF provider APIs.       │
├─────────────────────────┴─────────────────────────┴─────────────────────────┤
│        CLOSED-LOOP TELEMETRY & AUTOMATED FAIL-SAFE ROLLBACK                 │
│ Real-time Digital Twin ingestion; autonomous anomaly halt and dual-bank     │
│ spacecraft memory rollback upon telemetry regression.                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                 ORBITAL- & PASS-AWARE DETERMINISTIC DELIVERY                │
│ Native SGP4 ephemeris propagation, ground track projection, nadir angles,   │
│ and power/thermal constraints baked directly into deployment scheduling.    │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **Safety Before Speed:** Speed of deployment is meaningless if a single corrupted bit renders a satellite inoperable. In PLATO, no deployment proceeds without passing deterministic, cryptographically signed Release Assurance gates.
2. **End-to-End Cryptographic Traceability:** Every action—from local commit to CI build, multi-party cryptographic sign-off, ground-station uplink, and in-orbit verification—is permanently anchored in an immutable audit ledger.
3. **Vendor & Hardware Neutrality:** PLATO provides modular abstraction layers interfacing seamlessly with commercial Ground Station-as-a-Service providers (AWS Ground Station, KSAT, Leaf Space, Viasat) and standard spacecraft buses/payload architectures via open APIs.
4. **Closed-Loop Telemetry & Autonomous Rollback:** Real-time health signals (power margins, thermal limits, attitude drift, CPU/memory quotas) are continuously ingested into PLATO's Digital Twin. If a post-update anomaly is detected, PLATO immediately halts fleet rollouts and commands spacecraft fallback to a known-good dual-bank memory slot.
5. **Orbital- & Pass-Aware Scheduling:** Terrestrial CD tools assume infinite, low-latency broadband connectivity. Spacecraft software delivery operates over intermittent, Doppler-shifted 10-minute contact windows with asymmetric uplink budgets. PLATO natively couples orbital kinematics with deployment execution queues.

---

## Competitive Landscape & Alternatives Analysis

To understand PLATO's market positioning, we analyze the four existing approaches teams attempt to use today:

### 1. In-House Custom Scripts & Ad-Hoc Glue Code
* **How it works:** Flight software engineers build bespoke shell, Python, or Perl scripts that bundle binaries and trigger manual ground-station uploads via SFTP or CLI.
* **Why it fails:** Zero architectural standardization. Impossible to scale across multi-plane constellations. No cryptographic gating, no centralized role-based access control (RBAC), and high bus-factor vulnerability when key engineers depart.

### 2. Traditional Satellite Command & Control (C2) / Telemetry Systems
*(e.g., NASA COSMOS, Yamcs, OpenMCT, Major Tom, Ball Aerospace COSMOS)*
* **How it works:** Built to display real-time telemetry streams, monitor health sensors, and transmit individual telecommands (TC/TM packets) during a contact pass.
* **Why it fails:** Legacy C2 systems are **operation-centric, not software-centric**. They have zero understanding of Git version control, artifact repositories, Software Bills of Materials (SBOMs), canary deployment pipelines, or cryptographic CI/CD assurance policies. They are simply the pipe, not the release orchestrator.

### 3. Specialized Nanosatellite OTA Firmware Tools
*(e.g., SkyUpdate / UniLoader)*
* **How it works:** Niche, closed-source embedded utilities focused on the physical data-link transfer of delta-patches to individual CubeSat microcontrollers (MCUs) over narrow radio channels.
* **Why it fails:** Highly constrained to small CubeSat/university experiments. They lack constellation-scale release orchestration, multi-party approval workflows, multi-satellite fleet coordination, risk modeling, and enterprise DevSecOps governance. *(PLATO operates as the top-level orchestration plane that can coordinate with such lower-level protocols).*

### 4. Terrestrial CI/CD & Deployment Engines
*(e.g., ArgoCD, Spinnaker, GitLab CI/CD, Octopus Deploy, LaunchDarkly)*
* **How it works:** Industry-standard tools for deploying microservices to Kubernetes clusters or web servers over reliable fiber networks.
* **Why it fails:** Terrestrial tools assume 99.999% persistent TCP/IP connectivity, sub-second latency, and immediate rollback triggers. In space, networks are **Delay-Tolerant (DTN)**, links are intermittent, and a failed rollout cannot be fixed with a simple `kubectl rollout undo` without calculating ground-station contact cones and spacecraft orbital geometry.

### Comprehensive Comparison Matrix

| Capability / Dimension | Ad-Hoc In-House Scripts | Legacy Space C2 (COSMOS, Yamcs) | CubeSat OTA (SkyUpdate) | Terrestrial CD (ArgoCD, Spinnaker) | **PLATO Platform** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Primary Domain Focus** | Task automation | Real-time TC/TM & Telemetry | CubeSat MCU Patching | Cloud Microservices | **Space DevSecOps & Fleet Delivery** |
| **Git / CI/CD Native Handoff** | ❌ Manual | ❌ Non-existent | ❌ Manual | ✅ Full GitOps | ✅ **First-Class Git Handoff & Webhooks** |
| **Orbital Mechanics & SGP4 Aware**| ❌ No | ⚠️ Partial (Pass tracker only) | ❌ No | ❌ Impossible | ✅ **Native SGP4 Ephemeris Engine** |
| **Pass Window & Ground Station Queuing** | ❌ Manual | ⚠️ Operator-driven | ❌ No | ❌ No (Assumes 24/7 LAN/WAN) | ✅ **Autonomous Asynchronous Queuing** |
| **Cryptographic Assurance & SBOM Gating**| ❌ Ad-hoc | ❌ None | ⚠️ Basic checksum | ⚠️ Terrestrial only | ✅ **Cryptographic Multi-Party RAE Pipeline** |
| **Canary & Phased Fleet Rollout** | ❌ None | ❌ Manual satellite-by-satellite | ❌ Single-craft only | ✅ Microservice canary | ✅ **Orbital-Plane & Fleet-Wide Canary** |
| **Closed-Loop Telemetry Fallback** | ❌ Manual | ⚠️ Human operator eyeball | ⚠️ Local dual-bank only | ⚠️ HTTP health checks | ✅ **Automated Digital Twin Anomaly Fallback** |
| **Multi-Party RBAC & Immutable Audit** | ❌ No | ⚠️ Basic session logs | ❌ No | ⚠️ Terrestrial logs | ✅ **Write-Once Cryptographic Audit Trail** |
| **Constellation Scalability (1,000+ Assets)**| ❌ Fails completely | ⚠️ High operator overhead | ❌ Nanosats only | ⚠️ Cloud servers only | ✅ **Stateless Rust Engine (High Throughput)** |
| **Architecture Openness & Extensibility** | ❌ Proprietary hack | ⚠️ Mixed | ❌ Closed-source | ✅ Open-source | ✅ **Modern Open Modular Monolith (Rust/Next.js)** |

---

## System Architecture Whitepaper

PLATO is designed as a high-performance **modular monolith** written in **Rust**, pairing absolute memory safety and zero-cost abstractions with an asynchronous event-driven core and a modern Next.js 14 mission control dashboard.

### High-Level Topology Diagram

```mermaid
flowchart TD
    subgraph Earth_CI ["1. Earth: Developer & CI/CD Pipeline"]
        DEV["Spacecraft Software Engineer"] -->|git push| GH["Git Repository / GitHub Actions"]
        GH -->|Build Binary & Sign Hash| ART["Artifact Store (S3 / Registry)"]
        GH -->|Webhook / API Payload| PLATO_API["PLATO API Gateway (:8080)"]
    end

    subgraph PLATO_Core ["2. PLATO Mission DevSecOps Control Plane"]
        PLATO_API --> ROE["Release Orchestration Engine (ROE)"]
        
        ROE -->|Evaluate Trust| RAE["Release Assurance Engine (RAE)"]
        RAE -.->|Verify Signature, SBOM, MISRA| RAE
        
        ROE -->|Assess Environment| DRA["Deployment Readiness Assessment (DRA)"]
        DRA -.->|Check Pass Window, Link & Lock| DRA
        
        ROE -->|Execute Plan| DEE["Deployment Execution Engine (DEE)"]
        
        subgraph Specialized_Engines ["Specialized Coordination Subsystems"]
            DT["Digital Twin Engine\n(crates/twin)"]
            ORB["SGP4 Orbital Kinematics\n(crates/orbit)"]
            PLAN["Mission Planning Engine\n(crates/planning)"]
            RISK["Multi-Factor Risk Assessment\n(crates/risk)"]
            COORD["Constellation Coordination\n(crates/coordination)"]
            ANALYTICS["Reliability & Analytics\n(crates/analytics)"]
        end
        
        DRA <--> ORB
        DRA <--> DT
        PLAN <--> RISK
        DEE <--> COORD
        
        OUTBOX["Transactional Outbox Pattern"] --> NATS["Event Bus (NATS JetStream)"]
        NATS --> WORKER["PLATO Asynchronous Workers"]
        PLATO_API <--> PG[(PostgreSQL 16 DB\nImmutable Audit)]
    end

    subgraph Mission_Ops ["3. Human Command & Mission Control"]
        WEB["Next.js 14 Mission Control HUD (:3000)"] <-->|REST & WebSockets| PLATO_API
        OPERATOR["Flight Director / Mission Commander"] -->|Multi-Party Cryptographic Approval| WEB
    end

    subgraph Ground_Network ["4. Ground Station Infrastructure (GSaaS)"]
        DEE -->|Chunked Pass Queue| GS_ADAPT["Ground Station Connectors\n(AWS GS, KSAT, Leaf Space)"]
        GS_ADAPT -->|Uplink RF Transmission| GS_ANTENNA["Ground Station Antennas"]
    end

    subgraph Orbital_Fleet ["5. Space Segment: Constellation in Orbit"]
        GS_ANTENNA -->|S-Band / X-Band Uplink| SAT1["Satellite Alpha (Canary)"]
        GS_ANTENNA -.->|Subsequent Passes| SAT2["Satellite Fleet (Plane B)"]
        SAT1 -->|Downlink Telemetry| GS_ANTENNA
        GS_ANTENNA -->|Telemetry Ingestion| DT
        
        subgraph Spacecraft_Avionics ["Spacecraft Safe Fallback Architecture"]
            ACTIVE_BANK["Active Memory Bank A\n(New Firmware)"]
            RECOVERY_BANK["Golden Recovery Bank B\n(Fail-Safe Image)"]
            WATCHDOG["Hardware Watchdog / Telemetry Beacon"]
            WATCHDOG -.->|Anomaly Detected| RECOVERY_BANK
        end
    end
```

---

### The 4 Core Architectural Coordination Engines

PLATO transitions every release through a formal, deterministic mathematical state machine governed by four core engines:

#### 1. Release Orchestration Engine (ROE) — `crates/release` & `crates/application/src/roe.rs`
The master coordinator. It tracks and enforces the lifecycle transitions:
$$\text{Draft} \longrightarrow \text{Submitted} \longrightarrow \text{Validated} \longrightarrow \text{Assured} \longrightarrow \text{Approved} \longrightarrow \text{Scheduled} \longrightarrow \text{InProgress} \longrightarrow \text{Verified} \mid \text{RolledBack}$$
* **Core Question Answered:** *"What is the current lifecycle state of this software release across the constellation?"*
* **Guarantee:** No release can jump directly to deployment without passing through all intermediary cryptographic and organizational gates.

#### 2. Release Assurance Engine (RAE) — `crates/assurance`
The gatekeeper of software integrity.
* **Core Question Answered:** *"Should this software release ever be permitted to run on a spacecraft?"*
* **Assurance Checks:**
  - `ArtifactIntegrityCheck`: Cryptographic SHA-256 / Ed25519 hash verification against the build manifest.
  - `SignatureVerificationCheck`: Validation of developer and CI build signing keys against the authorized key ring.
  - `PolicyComplianceCheck`: Automated validation of SBOMs, static analysis reports (e.g., MISRA-C/C++, Clippy), zero-vulnerability CVE thresholds, and orbital safety policies.

#### 3. Deployment Readiness Assessment (DRA) — `crates/readiness`
The just-in-time operational gatekeeper executed prior to ground pass execution.
* **Core Question Answered:** *"Is today and this upcoming orbital window the right time to deploy this already-assured release?"*
* **Operational Readiness Checks:**
  - `DeploymentWindowCheck`: Verifies that the spacecraft is on an orbital trajectory intersecting a scheduled ground station contact cone with sufficient duration for the uplink budget.
  - `ConcurrentDeploymentCheck`: Ensures no conflicting telecommand sequences, orbit-raising maneuvers, or payload operations are scheduled simultaneously.
  - `DependencyCheck`: Verifies that target spacecraft hardware revisions, FPGA bitstreams, and current bootloader versions satisfy all dependency prerequisites.

#### 4. Deployment Execution Engine (DEE) — `crates/deployment`
The execution runtime translating approved plans into atomic delivery workflows.
* **Core Question Answered:** *"How do we reliably transmit, apply, and verify the binary over an intermittent, band-limited space link?"*
* **Pipeline Execution Steps:**
  1. `PreDeploymentValidation`: Final link health check and ground station lock confirmation.
  2. `PackagePreparation`: Differential delta compression, packet fragmentation, and Reed-Solomon / LDPC forward error correction encoding.
  3. `TargetSelection`: Resolution of target spacecraft IDs, orbital planes, or canary partitions.
  4. `DeploymentExecution`: Delivery to GSaaS queue and tracking uplink progress.
  5. `PostDeploymentVerification`: Closed-loop telemetry listening for reboot health, memory bank status, and telemetry verification.

---

### Specialized Domain Subsystems

Beyond the core four engines, PLATO features dedicated domain modules built for orbital complexity:

* **Digital Twin Engine (`crates/twin`):** Maintains high-fidelity digital replicas of all fleet assets. Encapsulates physical state (battery SOC, solar array illumination, thermal bus temperatures), avionics health, and active software versions.
* **Orbital Kinematics & SGP4 Prediction (`crates/orbit`):** High-precision ephemeris propagation using standard SGP4/SDP4 models. Calculates ground track coordinates, nadir projection, AOS/LOS (Acquisition/Loss of Signal) timings, and elevation angles for ground stations globally.
* **Mission Planning & Resource Scheduling (`crates/planning`, `crates/resource`):** Calculates multi-satellite pass windows, balancing ground station operational costs against fleet update priorities and satellite energy budgets (ensuring updates are never attempted during orbital eclipse/battery depletion).
* **Multi-Factor Risk Assessment Engine (`crates/risk`):** Dynamically scores deployments across four risk vectors:
  - *Operational Risk:* Impact on ongoing mission science/payload operations.
  - *Orbital Risk:* Pass duration margins, Doppler shift severity, atmospheric drag variations.
  - *Resource Risk:* Spacecraft power consumption overhead during flashing and reboot.
  - *Communication Risk:* Ground station RF link margins, elevation angles, and weather attenuation.
* **Fleet Rollout Coordination (`crates/coordination`):** Orchestrates multi-phase constellation updates:
  - Phase 1: Canary deployment on 1–2 test spacecraft.
  - Phase 2: Soak period monitoring telemetry for anomalous reboot loops or power spikes.
  - Phase 3: Plane-by-plane rollout across orbital rings.
  - Phase 4: Fleet-wide synchronization.
* **Reliability & Analytics (`crates/analytics`):** Real-time aggregation of fleet adoption curves, Mean Time to Rollback (MTTR), uplink bit-error rates (BER), and deployment reliability KPIs.

---

### Architectural Design Rationale: Why This Stack?

| Technology Choice | Rationale & Space Engineering Tradeoffs |
| :--- | :--- |
| **Rust Language** | **Absolute Memory Safety & Predictability:** Space operations cannot afford garbage collection pauses, segmentation faults, or data races when processing thousands of concurrent telemetry packets and real-time orbital calculations. |
| **Modular Monolith** | **High Velocity Without Microservice Hell:** Allows strict domain crate boundaries (`crates/*`) with compile-time dependency isolation, while maintaining single-binary deployment simplicity, atomic database migrations, and zero intra-service network latency. |
| **PostgreSQL 16 + SQLx** | **ACID Guarantees & Compile-Time Checked SQL:** Financial and space mission logs require relational integrity. SQLx provides compile-time query verification against the database schema, eliminating runtime query syntax errors. |
| **Transactional Outbox Pattern** | **Zero Message Loss Across Network Drops:** Database state mutations and NATS event emissions are executed atomically in the same database transaction. An outbox background worker guarantees at-least-once message delivery even during sudden server reboots. |
| **NATS JetStream Event Bus** | **Resilient, Ultra-Low Latency Messaging:** High-throughput streaming fabric decoupling real-time telemetry ingestion from API request-response threads, supporting persistent stream replays for audit reconstruction. |
| **Next.js 14 (App Router) + React Query** | **Military-Grade Mission Control HUD:** Instant client-side state synchronization, server-side rendered security headers, and smooth 60fps SVG rendering of orbital maps and live satellite telemetry scrubs without external heavy GIS dependencies. |

---

## The Digital Twin Release Model (DTRM)

Traditional software delivery treats a build artifact as an ephemeral file discarded after installation. In space operations, software must be treated as a **persistent, queryable Digital Twin** that lives as long as the satellite remains in orbit.

A single PLATO Release Identifier (e.g., `REL-2027-0042`) encapsulates the complete, immutable provenance of the code:

```json
{
  "release_id": "REL-2027-0042",
  "mission_id": "MISSION-STARGAZER-LEO",
  "version": "v2.4.1-rc3",
  "provenance": {
    "git_repository": "git@github.com:AdityaDevXYZ/Plato.git",
    "git_commit_sha": "f8e58509c2a8b3d4e5f6172839405a1b2c3d4e5f",
    "author": "engineer@space-enterprise.com",
    "ci_build_id": "GHA-RUN-982341",
    "compiler_toolchain": "rustc 1.77.2 (aarch64-unknown-linux-musl)"
  },
  "cryptography": {
    "artifact_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "signature": "30450221008f...",
    "signer_public_key": "ed25519:pk_ops_sec_4102"
  },
  "assurance_record": {
    "sbom_checksum": "a8f5b2...",
    "static_analysis_pass": true,
    "memory_footprint_bytes": 1048576,
    "verified_by_rae": true,
    "approved_by": ["ops_lead@space-enterprise.com", "flight_director@space-enterprise.com"]
  },
  "fleet_adoption": {
    "target_count": 120,
    "installed_count": 84,
    "verified_count": 84,
    "rolled_back_count": 0,
    "state": "InFlightRollout"
  }
}
```

---

## Operational Workflow: Traditional vs. PLATO

| Phase | Traditional Spaceflight Method | **PLATO Automated DevSecOps Workflow** |
| :--- | :--- | :--- |
| **1. Build** | Developer builds locally; passes loose binary via USB or shared drive. | **Automated Webhook/CI:** Git commit triggers containerized build, generates SBOM, and registers release in PLATO. |
| **2. Verify** | Scattered test logs; manual code reviews and spreadsheets. | **Automated RAE Gates:** Static analysis, cryptographic signature, and security checks verified before signing. |
| **3. Approve** | Unofficial Slack/email threads, sign-off sheets, verbal confirmation. | **Cryptographic RBAC Gate:** Dual-key digital authorization tied permanently to the binary hash and audit log. |
| **4. Schedule** | Manual spreadsheet calculation of ground station pass times. | **SGP4 Pass Optimization:** Autonomous pass scheduling factoring in orbital geometry, elevation, and battery budgets. |
| **5. Deliver** | Operator manually uploads binary during a high-stress 10-minute pass. | **Asynchronous Pass-Aware Queue:** DEE handles packetization, forward error correction, and GSaaS handoff. |
| **6. Monitor** | Engineers manually parse raw telemetry hex dumps post-pass. | **Closed-Loop Digital Twin:** Automated telemetry ingestion, health verification, and instant dual-bank rollback on anomaly. |

---

## Exact Steps to Launch PLATO

PLATO is designed to run everywhere—from a developer's local laptop or Termux mobile environment to an enterprise Kubernetes cluster.

### Prerequisites

* **Operating System:** Linux, macOS, or Android (Termux)
* **Rust Toolchain:** Stable Rust (1.77+) with `cargo` (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
* **Node.js:** Node.js 18+ and `npm`
* **Database:** PostgreSQL 14+ running locally or accessible via network

---

### Option A: One-Command Automated Startup (Recommended)

PLATO includes a self-healing startup script (`start.sh`) that validates background database daemons, initializes databases if missing, exports user-specific environment variables, and launches the API:

```bash
# 1. Clone the repository
git clone https://github.com/AdityaDevXYZ/Plato.git
cd Plato

# 2. Execute the automated environment launcher
bash start.sh
```

To run the Next.js Mission Control Frontend:
```bash
cd apps/web
npm install
npm run dev
```

* Backend API: `http://localhost:8080` (or `http://localhost:8080/health`)
* Frontend Mission Control HUD: `http://localhost:3000`

---

### Option B: Manual Step-by-Step Developer Launch

#### 1. Setup PostgreSQL Database
```bash
# Install and initialize PostgreSQL (if not already running)
# macOS (Homebrew):
brew install postgresql && brew services start postgresql

# Linux (Debian/Ubuntu):
sudo apt install postgresql && sudo systemctl start postgresql

# Termux (Android):
pkg install -y postgresql
initdb -D $PREFIX/var/lib/postgresql
pg_ctl -D $PREFIX/var/lib/postgresql -l logfile start

# Create the dedicated PLATO database
createdb plato
```

#### 2. Configure Environment Variables
Copy the example environment configuration into the project root:
```bash
cp .env.example .env
```
Ensure your `PLATO__DB__URL` is configured in `.env` or exported in your shell:
```bash
export PLATO__DB__URL="postgres://postgres:postgres@localhost:5432/plato"
# For Termux users:
export PLATO__DB__URL="postgres://$(whoami)@localhost/plato"
```

#### 3. Compile and Run the Backend API Gateway
```bash
# Verify workspace compilation
cargo check --workspace

# Run the API server with logging enabled
RUST_LOG=info cargo run -p api
```
*The API server will automatically run database migrations on boot and start listening on `0.0.0.0:8080`.*

#### 4. Launch the Mission Control Web App
In a separate terminal window:
```bash
cd apps/web
npm install
npm run dev
```
Open **`http://localhost:3000`** in your browser. You will see the interactive 2D Equirectangular SGP4 Orbital Map, Satellite Constellation Explorer, and Deployment Queues.

---

### Option C: Cloud-Native & Kubernetes (Production Deployment)

PLATO is packaged for enterprise cloud environments via multi-stage Dockerfiles and Kubernetes manifests.

#### 1. Build Multi-Stage Docker Containers
```bash
# Build Rust Backend Monolith
docker build -t ghcr.io/adityadevxyz/plato-api:latest -f infrastructure/docker/backend.Dockerfile .

# Build Next.js Mission Control Dashboard
docker build -t ghcr.io/adityadevxyz/plato-web:latest -f infrastructure/docker/frontend.Dockerfile .
```

#### 2. Deploy via Kubernetes (Kustomize)
```bash
# Apply the complete production stack (Postgres, NATS, Redis, Ingress, Deployments)
kubectl apply -k infrastructure/k8s/base
```

The Kubernetes manifests in `infrastructure/k8s/base` automatically provision:
* `namespace.yaml`: Dedicated `plato-system` namespace.
* `postgres.yaml` & `redis.yaml`: Highly available state storage with persistent volume claims.
* `nats.yaml`: NATS JetStream event bus for asynchronous pipeline execution.
* `backend-deployment.yaml`: Horizontal Pod Autoscaler (HPA)-ready Axum API pods with liveness and readiness health probes.
* `frontend-deployment.yaml`: Stateless Next.js SSR containers.
* `ingress.yaml`: TLS-terminated reverse proxy routing `/api/*` to Rust and `/` to Next.js.

---

## API Reference & Endpoint Tour

PLATO provides a comprehensive REST and WebSocket API documented with OpenAPI/Swagger:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          CORE PLATO API ROUTES                              │
├───────────────────────┬─────────────────────────────────────────────────────┤
│ GET  /health          │ Aggregated health probe (Database, Memory, Uptime)  │
│ GET  /health/live     │ Kubernetes Liveness Probe                           │
│ GET  /health/ready    │ Kubernetes Readiness Probe                          │
├───────────────────────┼─────────────────────────────────────────────────────┤
│ POST /api/v1/auth/login    │ Authenticate operator and issue JWT           │
│ GET  /api/v1/auth/me       │ Get current authenticated operator session    │
├───────────────────────┼─────────────────────────────────────────────────────┤
│ POST /api/v1/releases      │ Ingest new release bundle from CI/CD webhook  │
│ GET  /api/v1/releases/:id  │ Query Digital Twin Release Model provenance   │
│ POST /api/v1/releases/:id/validate   │ Trigger automated syntax validation │
│ POST /api/v1/releases/:id/assure     │ Run Release Assurance Engine (RAE)  │
│ POST /api/v1/releases/:id/approve    │ Multi-party cryptographic sign-off  │
│ POST /api/v1/releases/:id/readiness  │ Evaluate Deployment Readiness (DRA) │
│ POST /api/v1/releases/:id/deploy     │ Dispatch to Execution Engine (DEE)  │
├───────────────────────┼─────────────────────────────────────────────────────┤
│ GET  /api/v1/twin/satellites         │ Live state of all constellation assets│
│ GET  /api/v1/twin/satellites/:id     │ Telemetry, battery, CPU, and software│
│ GET  /api/v1/orbits/:id/prediction   │ SGP4 ephemeris pass predictions      │
│ GET  /api/v1/orbits/windows          │ Contact window calculation by station│
├───────────────────────┼─────────────────────────────────────────────────────┤
│ POST /api/v1/plans/:id/risk          │ Multi-factor deployment risk score   │
│ POST /api/v1/plans/:id/coordinate    │ Start phased canary rollout session  │
│ GET  /api/v1/analytics/reliability   │ Constellation MTTR & adoption curves │
└───────────────────────┴─────────────────────────────────────────────────────┘
```

---

## Security, Cryptography & Fail-Safe Assurance Model

1. **Zero-Trust Ground Station Connectors:** Ground stations are treated as untrusted transport mediums. Spacecraft payloads are signed end-to-end on Earth using asymmetric Ed25519 / RSA-4096 keys. Ground stations cannot alter payload bytes without invalidating the onboard signature verification.
2. **Dual-Bank Spacecraft Memory Architecture:** Satellite avionics configured for PLATO utilize dual-bank non-volatile flash memory (Bank A and Bank B). The existing stable image is preserved in Bank B while the update is flashed to Bank A.
3. **Autonomous Hardware Watchdog Handshake:** Following reboot on Bank A, the spacecraft's flight software must establish a nominal telemetry handshake with PLATO within a pre-configured watchdog timeout window (e.g., 30 minutes). If no valid acknowledgment is confirmed, the onboard hardware watchdog automatically reverts to Bank B.
4. **Immutable Audit Ledger:** All lifecycle events (`ReleaseCreated`, `AssurancePassed`, `ApprovedByOfficer`, `UplinkDispatched`, `TelemetryVerified`) are written to an append-only, tamper-evident PostgreSQL audit table with sequential transaction identifiers.

---

## Repository Structure & Workspace Tour

```
plato/
├── Cargo.toml                       # Cargo Workspace configuration (Resolver v2)
├── start.sh                         # Self-healing environment launcher
├── apps/                            # Executable Applications
│   ├── api/                         # Axum REST & WebSocket API Gateway Monolith
│   ├── cli/                         # PLATO Operator Command-Line Interface
│   ├── worker/                      # Asynchronous Queue & Pass Execution Worker
│   └── web/                         # Next.js 14 Mission Control Dashboard (HUD)
├── crates/                          # Domain Crates (Strict Architectural Boundaries)
│   ├── analytics/                   # Deployment velocity, reliability, and MTTR
│   ├── application/                 # Orchestration services (ROE, RAE, DRA, DEE)
│   ├── assurance/                   # Cryptographic signing, SBOM, and policy checks
│   ├── audit/                       # Write-once immutable audit event logging
│   ├── common/                      # Shared error types, pagination, and macros
│   ├── config/                      # Hierarchical environment & file configuration
│   ├── coordination/                # Multi-satellite rollout coordination & canaries
│   ├── deployment/                  # Execution pipeline & rollback implementations
│   ├── eventbus/                    # In-memory & NATS JetStream event publication
│   ├── identity/                    # RBAC, user authorization, and JWT processing
│   ├── mission/                     # Mission domain models and metadata
│   ├── notification/                # Operator alerting (Slack, PagerDuty, Webhooks)
│   ├── orbit/                       # SGP4 ephemeris mechanics & pass predictions
│   ├── organization/                # Multi-tenant organization boundaries
│   ├── planning/                    # Mission planning and pass window calculation
│   ├── policy/                      # Automated compliance and deployment policies
│   ├── project/                     # Project grouping and permission scopes
│   ├── readiness/                   # Just-in-time pass and environmental validation
│   ├── release/                     # Release entity lifecycle and state machine
│   ├── resource/                    # Spacecraft power, thermal, and link budgets
│   ├── risk/                        # Multi-factor operational risk scoring pipeline
│   ├── storage/                     # PostgreSQL SQLx repositories and migrations
│   ├── telemetry/                   # OpenTelemetry tracing and metrics instrumentation
│   ├── twin/                        # Digital Twin spacecraft fleet state engine
│   └── types/                       # Shared domain primitives and newtypes
├── infrastructure/                  # Production Infrastructure
│   ├── docker/                      # Multi-stage Dockerfiles (Backend & Frontend)
│   └── k8s/base/                    # Kubernetes manifests (Kustomize, NATS, Postgres)
└── docs/                            # Deep-dive architecture and design records
```

---

## Verification & Test Suites

The PLATO workspace incorporates comprehensive unit and integration test suites utilizing in-memory test doubles, mock ground stations, and deterministic telemetry generators:

```bash
# Run all workspace unit tests
cargo test --workspace

# Run tests with full backtrace
RUST_BACKTRACE=1 cargo test --workspace

# Run static analysis and clippy checks
cargo clippy --workspace --all-targets -- -D warnings

# Verify code formatting
cargo fmt --all -- --check
```

---

## Product Vision & Roadmap: The Path to Constellation Autonomy

Space exploration and commercial orbital infrastructure are undergoing the same transformation computing underwent 25 years ago: moving from custom mainframes to scalable, networked cloud clusters.

PLATO is positioning itself as **the standard software delivery backbone for commercial space**:

* [x] **Milestone 1: Core DevSecOps Engine (Current)** — Release Orchestration Engine (ROE), Cryptographic Assurance (RAE), SGP4 Orbital Kinematics, Next.js Mission Control HUD, and PostgreSQL Event Ledger.
* [ ] **Milestone 2: Hardware-in-the-Loop (HITL) Connectors** — Standardized adapters for FlatSat testbeds and popular commercial flight computers (OBCs).
* [ ] **Milestone 3: Space Packet Protocol & CCSDS Compliance** — Native integration with CCSDS Space Data Link Protocols, SLE (Space Link Extension), and CSP (Cubesat Space Protocol).
* [ ] **Milestone 4: Turnkey GSaaS Integrations** — Native one-click integrations with AWS Ground Station, KSATlite, Leaf Space, and Viasat Real-Time Earth.
* [ ] **Milestone 5: Autonomous In-Orbit Mesh Relays** — Inter-Satellite Optical Link (ISL) peer-to-peer distribution: uplinking code to a single primary satellite and allowing the constellation to autonomously disseminate, verify, and apply the release across the orbital shell.

---

## License

This project is licensed under the Apache 2.0 / MIT Dual License. See [LICENSE](LICENSE) for details.

---

<div align="center">

**Built for the next century of space operations.**  
*For inquiries, demos, or partnership requests: nectrragroup@gmail.com*

</div>
