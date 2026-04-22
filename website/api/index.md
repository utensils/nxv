# HTTP API

nxv includes a REST API server for programmatic access.

## Public Instance

A public instance is available for testing and light usage:

- **Web UI:** [nxv.urandom.io](https://nxv.urandom.io/)
- **API Docs:** [nxv.urandom.io/docs](https://nxv.urandom.io/docs)
- **API Base:** `https://nxv.urandom.io/api/v1`

::: tip Try it now You can use the public API directly without setting up your
own server:

```bash
curl "https://nxv.urandom.io/api/v1/search?q=python&limit=5"
```

:::

## Self-Hosted

To run your own instance, start the server with `nxv serve`.

### Base URL

```
http://localhost:8080/api/v1
```

## Authentication

No authentication required. Rate limiting can be enabled per IP address with
`--rate-limit`.

## Response Wrapper

All responses use a standard wrapper:

```json
{
  "data": [ ... ],
  "meta": {
    "total": 42,
    "limit": 50,
    "offset": 0,
    "has_more": false
  }
}
```

The `meta` field is present for paginated list responses and omitted for
single-item responses.

## Endpoints

### Search Packages

```
GET /api/v1/search
```

**Query Parameters:**

| Parameter | Type    | Description                     |
| --------- | ------- | ------------------------------- |
| `q`       | string  | Search query (required)         |
| `version` | string  | Version filter (prefix match)   |
| `exact`   | boolean | Exact name match                |
| `license` | string  | License filter                  |
| `sort`    | string  | Sort order: date, version, name |
| `reverse` | boolean | Reverse sort                    |
| `limit`   | integer | Max results (default: 50)       |
| `offset`  | integer | Results to skip (default: 0)    |

::: tip Description Search Description search uses a separate endpoint:
`GET /api/v1/search/description?q=<query>&limit=50&offset=0` :::

**Example:**

```bash
curl "http://localhost:8080/api/v1/search?q=python&version=3.11&limit=5"
```

**Response:**

```json
{
  "data": [
    {
      "id": 1,
      "attribute_path": "python311",
      "name": "python311",
      "version": "3.11.4",
      "description": "A high-level dynamically-typed programming language",
      "license": "Python-2.0",
      "first_commit_hash": "abc123...",
      "first_commit_date": "2023-06-15T00:00:00Z",
      "last_commit_hash": "def456...",
      "last_commit_date": "2023-12-01T00:00:00Z"
    }
  ],
  "meta": {
    "total": 42,
    "limit": 5,
    "offset": 0,
    "has_more": true
  }
}
```

### Get Package Versions

```
GET /api/v1/packages/{attr}
```

Returns all version records for a package.

**Example:**

```bash
curl "http://localhost:8080/api/v1/packages/python311"
```

### Get Specific Version

```
GET /api/v1/packages/{attr}/versions/{version}
GET /api/v1/packages/{attr}/versions/{version}/first
GET /api/v1/packages/{attr}/versions/{version}/last
```

Get all records, the first occurrence, or last occurrence of a specific version.

**Example:**

```bash
curl "http://localhost:8080/api/v1/packages/python311/versions/3.11.4/first"
```

### Version History

```
GET /api/v1/packages/{attr}/history
```

Returns version history entries with first/last seen dates.

**Example:**

```bash
curl "http://localhost:8080/api/v1/packages/python311/history"
```

**Response:**

```json
{
  "data": [
    {
      "version": "3.11.4",
      "first_seen": "2023-06-15T00:00:00Z",
      "last_seen": "2023-12-01T00:00:00Z",
      "is_insecure": false
    },
    {
      "version": "3.11.3",
      "first_seen": "2023-04-05T00:00:00Z",
      "last_seen": "2023-06-14T00:00:00Z",
      "is_insecure": false
    }
  ]
}
```

### Index Statistics

```
GET /api/v1/stats
```

**Response:**

```json
{
  "data": {
    "total_ranges": 2800000,
    "unique_names": 95000,
    "unique_versions": 450000,
    "oldest_commit_date": "2017-01-01T00:00:00Z",
    "newest_commit_date": "2024-01-15T00:00:00Z",
    "last_indexed_commit": "abc123...",
    "last_indexed_date": "2024-01-15T00:00:00Z"
  }
}
```

### Health Check

```
GET /api/v1/health
```

**Response:**

```json
{
  "status": "ok",
  "version": "0.1.6",
  "index_commit": "abc123..."
}
```

### Metrics

```
GET /api/v1/metrics
```

Returns server, database, rate-limit, runtime, latency, and per-minute activity
metrics. Intended for monitoring dashboards; responses are never cached.

**Example:**

```bash
curl "http://localhost:8080/api/v1/metrics"
```

## Error Responses

All errors return a JSON object:

```json
{
  "code": "NOT_FOUND",
  "message": "Package 'foobar' not found"
}
```

**HTTP Status Codes:**

| Code | Description                      |
| ---- | -------------------------------- |
| 200  | Success                          |
| 400  | Bad request (invalid parameters) |
| 404  | Not found                        |
| 429  | Rate limited                     |
| 500  | Internal server error            |

## OpenAPI Documentation

Interactive API documentation is available at:

```
http://localhost:8080/docs
```

## CORS

Enable CORS for browser access:

```bash
# All origins
nxv serve --cors

# Specific origins
nxv serve --cors-origins "https://example.com,https://app.example.com"
```

## Request Headers

| Header         | Description                                                 |
| -------------- | ----------------------------------------------------------- |
| `X-Request-ID` | Correlation ID for tracing (auto-generated if not provided) |

The server echoes back the request ID in responses for distributed tracing.
