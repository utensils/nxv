# Troubleshooting

Common issues and solutions when using nxv.

## Search Issues

### No results found

1. **Check spelling**: Exact matches (`--exact`) are case-sensitive

   ```bash
   nxv search Python --exact  # Wrong
   nxv search python --exact  # Correct
   ```

2. **Try partial match**: Use prefix search

   ```bash
   nxv search pyth
   ```

3. **Search descriptions**: The package might have a different name

   ```bash
   nxv search "image editor" --desc
   ```

4. **Update index**: Your index might be outdated
   ```bash
   nxv update
   ```

### Search is slow

The first search loads the bloom filter and warms the database cache. Subsequent
searches are fast. If searches remain slow:

1. Check disk I/O (SSD recommended)
2. Check `NXV_DB_PATH` isn't on a network drive

## Update Issues

### Download fails

1. **Check network**: Ensure GitHub is accessible
2. **Retry**: Transient failures are common
   ```bash
   nxv update
   ```
3. **Force full download**: Skip delta updates
   ```bash
   nxv update --force
   ```

### Signature verification failed

1. **Check clock**: Signature validation requires accurate time
2. **Skip verification** (not recommended for production):
   ```bash
   NXV_SKIP_VERIFY=1 nxv update
   ```

### Incompatible index

```
Incompatible index: index requires schema version N but this build only supports up to M
```

This means your nxv binary is too old for the published index. `nxv update`
handles it automatically: on this error it runs the binary self-update check
before exiting.

- **Local installs** (install.sh, manual download): the binary is replaced in
  place after SHA-256 verification — re-run `nxv update` to fetch the index.
- **Managed installs** (Nix, cargo, Homebrew): the matching upgrade command is
  printed instead; run it, then re-run `nxv update`.

Pass `--no-self-update` (or set `NXV_NO_SELF_UPDATE`) to skip the binary check
if updates are managed externally.

### Disk full

The compressed index download is ~220MB and unpacks to ~2.1GB of disk space:

- Linux: `~/.local/share/nxv/`
- macOS: `~/Library/Application Support/nxv/`

## Server Issues

### Port already in use

```bash
# Use different port
nxv serve --port 3000

# Find what's using port 8080
lsof -i :8080
```

### CORS errors in browser

```bash
# Enable CORS for all origins
nxv serve --cors

# Or specific origins
nxv serve --cors-origins "http://localhost:3000"
```

### Connection refused

1. **Check bind address**: Default is localhost only

   ```bash
   nxv serve --host 0.0.0.0  # Listen on all interfaces
   ```

2. **Check firewall**: Ensure port is open

## Indexer Issues

The indexer ingests channel-release snapshots from releases.nixos.org — it does
not need a nixpkgs checkout.

### Failed releases

A release that fails to download or parse is retried automatically with
exponential backoff; after repeated failures it is parked as `failed` in the
ledger. To force a retry of parked releases:

```bash
nxv index --retry-failed
```

### Indexing stuck

1. **Check logs**:

   ```bash
   nxv -v index
   ```

2. **Resume**: Interrupted runs resume automatically — ingestion is tracked per
   release, so the next run picks up any remaining pending releases.

3. **Narrow the range**: Limit a run to a date window while debugging:

   ```bash
   nxv index --since 2024-01-01 --until 2024-02-01
   ```

### Disk full during indexing

Snapshots are streamed during parsing and never written to disk. Only
`--backfill-evals` and `--head-eval` evaluate with Nix, which grows the Nix
store:

```bash
# Manual cleanup
nix-collect-garbage -d
```

## Getting Help

1. **Check GitHub Issues**:
   [github.com/utensils/nxv/issues](https://github.com/utensils/nxv/issues)
2. **Open an issue**: Include:
   - nxv version (`nxv --version`)
   - Operating system
   - Command that failed
   - Error message
   - Debug logs if available
