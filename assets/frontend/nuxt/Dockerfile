FROM node:lts-slim AS build

RUN npm install -g pnpm@{{PNPM}}

WORKDIR /app

COPY . .
RUN pnpm install --frozen-lockfile && pnpm build


FROM node:lts-slim AS runtime

WORKDIR /app
COPY --from=build /app/.output ./.output

USER node

ENV NODE_ENV=production \
    NITRO_HOST=0.0.0.0 \
    NITRO_PORT=3000

EXPOSE 3000

CMD ["node", ".output/server/index.mjs"]
