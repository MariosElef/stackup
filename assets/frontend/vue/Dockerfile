FROM node:lts-slim AS build

RUN npm install -g pnpm@{{PNPM}}

WORKDIR /app

COPY . .
RUN pnpm install --frozen-lockfile && pnpm build


FROM nginx:alpine

COPY --from=build /app/dist /usr/share/nginx/html

EXPOSE 80
