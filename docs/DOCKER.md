# Docker Setup for IPE Documentation

This directory includes Docker Compose configuration for running the IPE documentation site with nginx.

## 🐳 Quick Start

```bash
# Start the nginx server
just docs-serve

# View at http://localhost:8080
open http://localhost:8080

# Stop the server
just docs-stop
```

## 📋 Available Commands

### Server Management

```bash
# Start nginx server in background
just docs-serve

# Stop nginx server
just docs-stop

# View server logs
just docs-logs

# Restart server (useful after content changes)
just docs-stop && just docs-serve
```

### Testing

```bash
# Run Playwright tests (starts/stops Docker automatically)
just test-pages

# Open browser for manual testing (Docker nginx)
just test-pages-headed

# Quick validation tests (no Docker needed)
just test-pages-quick

# Full test suite in Docker containers
just test-pages-docker
```

## 🏗️ Architecture

### docker-compose.yml

Defines two services:

1. **docs** (always runs)
   - nginx:alpine container
   - Serves static files from docs/ directory
   - Available at http://localhost:8080
   - Includes health checks

2. **playwright** (test profile only)
   - Official Playwright container
   - Runs automated browser tests
   - Only starts with `--profile test`

### nginx.conf

Custom nginx configuration with:
- Static file serving
- Gzip compression
- Security headers
- CORS headers (for local development)
- Cache control for assets
- Pretty URL support

### Dockerfile

Optional standalone image build:
- Based on nginx:alpine (minimal size)
- Copies static files into image
- Includes health checks
- Can be built and deployed independently

## 🔧 Advanced Usage

### Manual Docker Compose

```bash
cd docs

# Start services
docker-compose up -d

# View logs
docker-compose logs -f docs

# Stop services
docker-compose down

# Rebuild and start
docker-compose up -d --build

# Run tests
docker-compose --profile test up --abort-on-container-exit
```

### Build Standalone Image

```bash
cd docs

# Build image
docker build -t ipe-docs:latest .

# Run container
docker run -d -p 8080:8080 --name ipe-docs ipe-docs:latest

# Stop and remove
docker stop ipe-docs
docker rm ipe-docs
```

### Custom Port

To use a different port, edit `docker-compose.yml`:

```yaml
services:
  docs:
    ports:
      - "3000:8080"  # Maps host:3000 to container:8080
```

## 🔍 Health Checks

The nginx container includes health checks:

```bash
# Check container health
docker inspect ipe-docs | grep -A 10 Health

# View health check logs
docker logs ipe-docs 2>&1 | grep health
```

## 🐛 Troubleshooting

### Port Already in Use

```bash
# Find process using port 8080
lsof -ti:8080

# Kill process
kill $(lsof -ti:8080)

# Or use different port in docker-compose.yml
```

### Container Won't Start

```bash
# Check logs
just docs-logs

# Or
docker-compose logs docs

# Rebuild container
docker-compose down
docker-compose up -d --build
```

### Files Not Updating

The docs directory is mounted as a volume, so changes should appear immediately. If not:

```bash
# Restart nginx to reload config
docker-compose restart docs

# Or fully restart
just docs-stop && just docs-serve
```

### Permission Issues

If you get permission errors:

```bash
# Ensure files are readable
chmod -R 755 .

# Or run with user permissions
docker-compose run --user $(id -u):$(id -g) docs
```

## 📊 Performance

nginx:alpine is very lightweight:
- Image size: ~40MB
- Memory usage: ~5-10MB
- CPU usage: Minimal (<1%)
- Startup time: <1 second

Perfect for local development and testing.

## 🚀 Production Deployment

While this setup is great for local development, for production:

1. **Use the Dockerfile** to create a standalone image:
   ```bash
   docker build -t ipe-docs:v1.0.0 .
   docker push your-registry/ipe-docs:v1.0.0
   ```

2. **Or deploy to GitHub Pages** (recommended for static docs):
   - No Docker needed
   - Free hosting
   - Global CDN
   - Automatic HTTPS

3. **Or use Kubernetes** for larger deployments:
   ```bash
   kubectl create deployment ipe-docs --image=ipe-docs:v1.0.0
   kubectl expose deployment ipe-docs --port=80 --target-port=8080
   ```

## 🔐 Security Notes

**For Local Development:**
- CORS headers are permissive (`Access-Control-Allow-Origin: *`)
- No authentication required
- Suitable for localhost only

**For Production:**
- Remove or restrict CORS headers in nginx.conf
- Add authentication if needed
- Use HTTPS (TLS termination)
- Consider rate limiting

## 📝 Files

- `docker-compose.yml` - Multi-service orchestration
- `Dockerfile` - Standalone image build
- `nginx.conf` - Web server configuration
- `.dockerignore` - Exclude files from image
- `DOCKER.md` - This file

## 💡 Tips

1. **Keep container running** during development:
   ```bash
   just docs-serve
   # Make changes to HTML/CSS/JS
   # Refresh browser to see changes
   ```

2. **Use volumes** for live reload (already configured):
   - Changes to files appear immediately
   - No need to rebuild container

3. **Check logs** if something doesn't work:
   ```bash
   just docs-logs
   ```

4. **Clean up** when done:
   ```bash
   just docs-stop
   docker system prune  # Optional: clean up unused images
   ```

## 🎯 Next Steps

1. Start the server: `just docs-serve`
2. Open browser: `open http://localhost:8080`
3. Run tests: `just test-pages`
4. Stop when done: `just docs-stop`

That's it! The Docker nginx setup provides a production-like environment for local development and testing.
