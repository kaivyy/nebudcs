# NebuDCS Production Operations & Deployment Runbook

## 1. Overview & Architecture Guarantees

NebuDCS is deployed as an enterprise-grade generic process simulation engine and DCS training platform.
The architecture enforces strict separation of concerns:

- **Authoritative Process Authority:** The Rust simulation runtime (`nebudcs-server`) is the sole process authority. Visual representations, web clients, and instructor consoles never mutate process physics directly without passing through deterministic command pipelines.
- **Generic Runtime:** Process units, equipment topologies, control loops, and alarms are completely data-driven. No plant-specific equipment IDs or refinery unit schemas are compiled into the core.
- **Deterministic Sim Boundaries:** The simulation loop runs on a dedicated high-priority thread with strict execution budgets (50ms timestep, 20Hz nominal rate).
- **Security & Integrity:** Constant-time cryptographic token verification, server-side Role-Based Access Control (RBAC), and SHA-256 backup bundle validation protect against tampering, privilege escalation, and data loss.

---

## 2. Server Installation & Systemd Deployment

### 2.1 Building the Production Binary
```bash
# Build optimized release binary
cargo build --release -p nebudcs-server

# Copy binary to system path
sudo cp target/release/nebudcs-server /usr/local/bin/
sudo chmod 755 /usr/local/bin/nebudcs-server
```

### 2.2 User & Directory Provisioning
```bash
# Create dedicated system user without login privileges
sudo useradd -r -s /bin/false -d /var/lib/nebudcs nebudcs

# Provision configuration and data directories
sudo mkdir -p /etc/nebudcs
sudo mkdir -p /var/lib/nebudcs/data
sudo mkdir -p /var/backups/nebudcs

# Secure permissions
sudo chown -R nebudcs:nebudcs /var/lib/nebudcs /var/backups/nebudcs
sudo chmod 700 /var/lib/nebudcs /var/backups/nebudcs
```

### 2.3 Environment Configuration
Copy the configuration template:
```bash
sudo cp deploy/nebudcs.env /etc/nebudcs/nebudcs.env
sudo chown root:nebudcs /etc/nebudcs/nebudcs.env
sudo chmod 640 /etc/nebudcs/nebudcs.env
```

Edit `/etc/nebudcs/nebudcs.env` to configure secure role tokens:
- `ADMIN_TOKEN`: Full administrative control (backup, restore, migrations, reload).
- `INSTRUCTOR_TOKEN`: Authoring scenarios, orchestrating training sessions, and injecting process upsets.
- `OPERATOR_TOKEN`: DCS console operation, setpoint tuning, manual valve positioning, alarm acknowledgement/shelving.
- `TRAINEE_TOKEN`: Operator-station console access with restricted permissions (cannot author scenarios, inject faults, or alter topology).

### 2.4 Enabling Systemd Service
```bash
sudo cp deploy/nebudcs.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nebudcs.service
sudo systemctl status nebudcs.service
```

---

## 3. Remote Operations via Tailscale Mesh Network

To connect distributed instructor consoles, remote operator stations, and engineering laptops without public IP exposure or complex VPN gateways, deploy a **Tailscale** overlay network:

1. Install Tailscale on the NebuDCS host:
   ```bash
   curl -fsSL https://tailscale.com/install.sh | sh
   sudo tailscale up --hostname=nebudcs-sim-01
   ```
2. In the Tailscale Admin Console, define Access Control Lists (ACLs) to enforce network segmentation:
   ```json
   {
     "acls": [
       // Only engineering & instructor machines can reach port 8080 admin endpoints
       {
         "action": "accept",
         "src": ["tag:instructor", "tag:engineer"],
         "dst": ["tag:simulator:8080"]
       },
       // Trainee station terminals can only connect to WebSocket and DCS console
       {
         "action": "accept",
         "src": ["tag:trainee"],
         "dst": ["tag:simulator:8080"]
       }
     ]
   }
   ```
3. Operators access the simulator securely using `http://nebudcs-sim-01.internal-mesh:8080`.

---

## 4. Backup, Restore & Disaster Recovery

### 4.1 Automated Daily Backups
Install `deploy/backup.sh` to `/usr/local/bin/nebudcs-backup.sh`:
```bash
sudo cp deploy/backup.sh /usr/local/bin/nebudcs-backup.sh
sudo chmod 750 /usr/local/bin/nebudcs-backup.sh
```
Add to crontab (`sudo crontab -e`):
```cron
0 2 * * * /usr/local/bin/nebudcs-backup.sh >> /var/log/nebudcs-backup.log 2>&1
```

### 4.2 Restoring from Backup Bundle
To restore a snapshot archive after server migration or disaster:
```bash
curl -X POST "http://localhost:8080/api/v1/admin/restore" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d @/var/backups/nebudcs/nebudcs_backup_20260907T020000Z.json
```
The server validates the cryptographic SHA-256 hash of each entity before writing to disk, preventing corrupt or tampered payloads from loading into runtime memory.

---

## 5. Schema Migration & Zero-Downtime Rolling Upgrade

When updating to new NebuDCS versions with modified project or scenario schemas:
1. Deploy new binary to `/usr/local/bin/nebudcs-server.new`.
2. Trigger the schema migration endpoint:
   ```bash
   curl -X POST "http://localhost:8080/api/v1/admin/migrate" \
     -H "Authorization: Bearer $ADMIN_TOKEN"
   ```
3. Replace binary and restart systemd service:
   ```bash
   sudo mv /usr/local/bin/nebudcs-server.new /usr/local/bin/nebudcs-server
   sudo systemctl restart nebudcs.service
   ```
4. Verify health probes:
   ```bash
   curl -f http://localhost:8080/api/v1/health/live
   curl -f http://localhost:8080/api/v1/health/ready
   curl -f http://localhost:8080/api/v1/metrics
   ```
