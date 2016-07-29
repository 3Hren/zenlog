#!/usr/bin/env python3

import json
import os
import socket;
import sys
import time
import threading

s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.connect(('127.0.0.1', 50031))

for i in range(10):
    load = {
        'severity': 'DEBUG',
        'levelno': 0,
        'message': 'le message',
        'timestamp': int(time.time() * 1e9),
        'pid': os.getpid(),
        'tid': threading.get_ident(),
    }
    load = json.dumps(load).encode('utf8')
    s.send(load)

load = {
    'severity': 'DEBUG',
    'levelno': 0,
    'message': 'le message',
    'timestamp': -1,
    'pid': os.getpid(),
    'tid': threading.get_ident(),
}
load = json.dumps(load).encode('utf8')
s.send(load)
