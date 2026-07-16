# Runbook: High CPU / 5xx Errors on Production API Gateway (continued)

**Runbook ID:** RB-SRE-004

---

## 3. Common Causes & Remediation

### Cause A: Connection Pool Exhaustion (most likely)
**Symptoms:** Connection pool at 100%, queue depth growing, upstream connection refused
**Trigger:** Recent config change increasing `connection_pool_size` without corresponding DB/backend capacity

**Remediation:**
1. Roll back `connection_pool_size` to previous value (50)
2. Restart user-service pods to drain stale connections
3. Monitor connection pool metrics for 5 minutes
4. If stable, investigate why backends can't handle 200 connections

### Cause B: Memory Leak After Deploy
**Symptoms:** Memory grows steadily after deploy, GC pressure, CPU high
**Remediation:**
1. Roll back to previous version
2. Notify engineering team of potential memory leak
3. Monitor memory after rollback

### Cause C: Database Connection Saturation
**Symptoms:** DB query timeouts, connection pool exhaustion on DB side
**Remediation:**
1. Check pg_stat_activity for long-running queries
2. Kill stuck queries if needed
3. Consider read replicas for read-heavy workloads

### Cause D: Traffic Spike
**Symptoms:** Request rate significantly above baseline, no recent changes
**Remediation:**
1. Verify auto-scaling is working
2. Check for DDoS or bot traffic
3. Consider rate limiting or WAF rules

## 3. Remediation

### Immediate (stop the bleeding)
- **Roll back config change** if connection_pool_size was recently increased
- **Roll back deploy** if a recent deploy correlates with the incident
- **Disable experimental feature flags** if they correlate
- **Restart affected services** if connection pool is stuck

### Short-term (restore stability)
- Scale up pods (auto-scaler should handle this)
- Increase rate limiting if traffic spike is abnormal
- Consider read replicas for DB load

### Long-term (prevent recurrence)
- Review connection pool sizing guidelines
- Add circuit breaker thresholds
- Add pre-deploy canary analysis
- Update runbook with new findings

## 4. Escalation

- If root cause is not found within 15 minutes, escalate to senior SRE
- If customer-impacting, notify engineering manager
- If data loss risk, notify DBA team
- If security concern, notify security team

## 5. Post-Incident

1. Ensure all monitoring is back to green
2. Write postmortem within 48 hours
3. Schedule incident review meeting
4. Update runbook with any new findings
5. File follow-up tickets for long-term fixes
