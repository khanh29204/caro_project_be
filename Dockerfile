# Stage 1: Cài tất cả dependencies (dev + prod) để build
FROM node:20-bullseye-slim AS deps
WORKDIR /app
COPY package.json yarn.lock ./
RUN yarn install --frozen-lockfile

# Stage 2: Build TypeScript
FROM node:20-bullseye-slim AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN yarn build

# Stage 3: Chỉ cài production dependencies
FROM node:20-bullseye-slim AS prod-deps
WORKDIR /app
COPY package.json yarn.lock ./
RUN yarn install --frozen-lockfile --production --no-progress --prefer-offline

# Stage 4: Runner
FROM node:20-alpine AS runner
WORKDIR /app

# Copy production node_modules
COPY --from=prod-deps /app/node_modules ./node_modules

# Copy build output + package.json
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/package.json ./
EXPOSE 3005
CMD ["node", "dist/server.js"]