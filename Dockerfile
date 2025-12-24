# Stage 1: Install dependencies
FROM oven/bun:1-alpine AS builder
WORKDIR /app

COPY package.json bun.lock* ./
# Cài đặt toàn bộ bao gồm cả devDependencies để phục vụ build (nếu có)
RUN bun install --frozen-lockfile

# Copy toàn bộ source và build (nếu bạn dùng TypeScript cần transpile hoặc gom file)
COPY . .

# Stage 2: Production runtime
FROM oven/bun:1-alpine AS runner
WORKDIR /app

# Chỉ copy những thứ cần thiết từ stage builder
# Nếu project của bạn chạy trực tiếp file .ts bằng bun, copy source và node_modules
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/src ./src
COPY --from=builder /app/package.json ./package.json

# Thiết lập môi trường production
ENV NODE_ENV=production
USER bun

EXPOSE 3005

CMD ["bun", "src/server.ts"]