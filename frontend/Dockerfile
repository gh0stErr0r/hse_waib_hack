FROM node:22-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
ARG VITE_APPLICATION_URL
ENV VITE_APPLICATION_URL=$VITE_APPLICATION_URL
ARG VITE_PRESENTATION_MODE=false
ENV VITE_PRESENTATION_MODE=$VITE_PRESENTATION_MODE
ARG VITE_MAP_PROVIDER=osm
ENV VITE_MAP_PROVIDER=$VITE_MAP_PROVIDER
ARG VITE_MAP_ATTRIBUTION
ENV VITE_MAP_ATTRIBUTION=$VITE_MAP_ATTRIBUTION
ARG VITE_MAP_TILES_URL=https://tile.openstreetmap.org/{z}/{x}/{y}.png
ENV VITE_MAP_TILES_URL=$VITE_MAP_TILES_URL
ARG VITE_YANDEX_MAPS_KEY
ENV VITE_YANDEX_MAPS_KEY=$VITE_YANDEX_MAPS_KEY
RUN npm run build
FROM nginx:1.28-alpine AS presentation
COPY nginx.presentation.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80

FROM nginx:1.28-alpine AS production
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
