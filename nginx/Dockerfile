FROM node:22-slim AS frontend-builder
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

FROM nginx:1.27-alpine
COPY nginx/nginx.conf /etc/nginx/nginx.conf
COPY nginx/certs /etc/nginx/certs
COPY --from=frontend-builder /app/build /usr/share/nginx/html
EXPOSE 80 443
