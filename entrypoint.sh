#!/bin/bash
cp $HOME/.aws/credentials_template $HOME/.aws/credentials
sed -i "s|AWS_ACCESS_KEY_ID|${AWS_ACCESS_KEY_ID}|" $HOME/.aws/credentials
sed -i "s|AWS_SECRET_ACCESS_KEY|${AWS_SECRET_ACCESS_KEY}|" $HOME/.aws/credentials
