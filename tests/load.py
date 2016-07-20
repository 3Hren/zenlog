#!/usr/bin/env python

import os
import sys
import json


load = {
    'message': 'le message',
    'pid': os.getpid(),
    'thread': 2,
    'severity': [0, 'DEBUG'],
}

load = json.dumps(load)

for i in xrange(100):
    sys.stdout.write(load)
    sys.stdout.flush()
