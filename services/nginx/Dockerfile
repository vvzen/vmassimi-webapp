FROM nginx:1.17-alpine

# Remove any preexisting NGINX conf
RUN rm /etc/nginx/nginx.conf
RUN rm /etc/nginx/conf.d/default.conf

# Copy my own config
COPY ./nginx.conf /etc/nginx/nginx.conf
