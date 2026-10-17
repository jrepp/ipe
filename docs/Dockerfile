FROM nginx:alpine

# Copy custom nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy static files
COPY *.html /usr/share/nginx/html/
COPY *.json /usr/share/nginx/html/
COPY *.md /usr/share/nginx/html/
COPY *.js /usr/share/nginx/html/

# Copy tests directory if exists
COPY tests /usr/share/nginx/html/tests/ 2>/dev/null || true

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:8080/ || exit 1

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
