# Troubleshooting

Common issues and solutions when using nxv.

## Search Issues

### No results found

1. **Check spelling**: Package names are case-sensitive

   ```bash
   nxv search Python  # Wrong
   nxv search python  # Correct
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

The first search loads the database and bloom filter into memory. Subsequent
searches are fast. If searches remain slow:

1. Check disk I/O (SSD recommended)
2. Ensure sufficient RAM (~500MB for index)
3. Check `NXV_DB_PATH` isn't on a network drive

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

### Disk full

The index requires ~100MB of disk space:

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

### Disk full during indexing

Nix store grows during indexing:

```bash
# Manual cleanup
nix-collect-garbage -d
```

### Indexing stuck

1. **Check logs**:

   ```bash
   nxv -v index --nixpkgs-path ./nixpkgs
   ```

2. **Reset and resume**:
   ```bash
   nxv reset --nixpkgs-path ./nixpkgs
   nxv index --nixpkgs-path ./nixpkgs
   ```

### Worker crashes

Workers may crash on certain commits:

```bash
# Skip problematic date range
nxv index --nixpkgs-path ./nixpkgs --since 2023-06-01

# Check worker logs
nxv -vv index --nixpkgs-path ./nixpkgs
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
