#!/bin/bash
cp /home/${APP_USER}/.aws/credentials_template /home/${APP_USER}/.aws/credentials
sed -i "s|AWS_ACCESS_KEY_ID|${AWS_ACCESS_KEY_ID}|" /home/${APP_USER}/.aws/credentials
sed -i "s|AWS_SECRET_ACCESS_KEY|${AWS_SECRET_ACCESS_KEY}|" /home/${APP_USER}/.aws/credentials
