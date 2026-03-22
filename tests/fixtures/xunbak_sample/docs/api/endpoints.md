# API Documentation

## Endpoints

### GET /api/status

Returns the current system status.

**Response:**
```json
{
  "status": "ok",
  "uptime_seconds": 12345,
  "version": "0.1.0"
}
```

### POST /api/backup

Triggers a new backup operation.

**Request:**
```json
{
  "source": "/path/to/source",
  "container": "/path/to/backup.xunbak",
  "compression": "zstd",
  "level": 1
}
```

**Response:**
```json
{
  "snapshot_id": "01HZQX5KPBX3M1234567890AB",
  "blob_count": 42,
  "total_bytes": 1048576,
  "duration_ms": 350
}
```

### GET /api/snapshots

Lists available snapshots.

**Response:**
```json
{
  "snapshots": [
    {
      "id": "01HZQX5KPBX3M1234567890AB",
      "created_at": "2026-03-22T10:30:00Z",
      "file_count": 42,
      "total_bytes": 1048576
    }
  ]
}
```

### POST /api/restore

Restores files from a snapshot.

**Request:**
```json
{
  "container": "/path/to/backup.xunbak",
  "target": "/path/to/restore",
  "mode": "all"
}
```
