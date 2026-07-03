FROM node:22-slim AS frontend-builder
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

FROM nginx:1.27-alpine
RUN apk add --no-cache openssl \
    && mkdir -p /etc/nginx/certs \
    && openssl req -x509 -nodes -days 825 -newkey rsa:2048 \
       -keyout /etc/nginx/certs/privkey.pem \
       -out /etc/nginx/certs/fullchain.pem \
       -subj "/CN=localhost" -addext "subjectAltName=DNS:localhost"
COPY nginx/nginx.conf /etc/nginx/nginx.conf
COPY --from=frontend-builder /app/build /usr/share/nginx/html
EXPOSE 80 443
