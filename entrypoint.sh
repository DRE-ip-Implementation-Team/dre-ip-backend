#!/bin/bash
sed -i "s|AWS_ACCESS_KEY_ID|${AWS_ACCESS_KEY_ID}|" /home/dreip/.aws/credentials
sed -i "s|AWS_SECRET_ACCESS_KEY|${AWS_SECRET_ACCESS_KEY}|" /home/dreip/.aws/credentials
