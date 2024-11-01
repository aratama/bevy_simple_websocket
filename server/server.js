// server example using ws library

import { WebSocket, WebSocketServer } from 'ws';

const wss = new WebSocketServer({ port: 8080 });

wss.on('connection', (ws, req) => {

  const ip = req.socket.remoteAddress;
  console.log("Client connected, ip address:", ip)

  ws.on('error', (e) => {
    console.error(e);
  });

  ws.on('open', () =>  {
    console.log('connected: ', ip);
  });
  
  ws.on('close', () => {
    console.log('disconnected: ', ip);
  });

  ws.on('message', (data, isBinary) => {
    if(!isBinary) {
      console.log('[client]', data.toString());
    }
    
    for(const client of wss.clients) {
      if (client !== ws && client.readyState === WebSocket.OPEN) {
        client.send(data, { binary: isBinary });
      }
    }

  });

  ws.send('hello from server');
});

console.log('Server started on port 8080');