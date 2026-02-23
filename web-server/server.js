const express = require('express');
const Database = require('better-sqlite3');
const path = require('path');

const app = express();
const PORT = process.env.PORT || 3000;
const DB_PATH = process.env.DB_PATH || 'staking_indexer.db';

// 连接数据库
let db;
try {
  db = new Database(DB_PATH, { readonly: true });
  console.log(`Database connected successfully: ${DB_PATH}`);
} catch (error) {
  console.error('Failed to connect to database:', error);
  process.exit(1);
}

// 静态文件服务
app.use(express.static(path.join(__dirname, 'public')));

// API 路由：获取所有 validators
app.get('/api/validators', (req, res) => {
  try {
    const stmt = db.prepare('SELECT address, pubkey FROM validators ORDER BY created_at DESC');
    const validators = stmt.all();
    res.json(validators);
  } catch (error) {
    console.error('Error fetching validators:', error);
    res.status(500).json({ error: 'Failed to fetch validators' });
  }
});

// API 路由：获取所有 delegators
app.get('/api/delegators', (req, res) => {
  try {
    const stmt = db.prepare('SELECT address, total_delegated, total_undelegated FROM delegators ORDER BY created_at DESC');
    const delegators = stmt.all();
    res.json(delegators);
  } catch (error) {
    console.error('Error fetching delegators:', error);
    res.status(500).json({ error: 'Failed to fetch delegators' });
  }
});

// API 路由：获取 events（支持过滤）
app.get('/api/events', (req, res) => {
  try {
    const { delegator_address, validator_address, event_type } = req.query;
    
    let query = 'SELECT delegator_address, validator_address, event_type, amount, shares, transaction_hash, block_number, block_timestamp FROM events WHERE 1=1';
    const params = [];
    
    if (delegator_address) {
      query += ' AND delegator_address = ?';
      params.push(delegator_address);
    }
    
    if (validator_address) {
      query += ' AND validator_address = ?';
      params.push(validator_address);
    }
    
    if (event_type !== undefined && event_type !== '') {
      query += ' AND event_type = ?';
      params.push(parseInt(event_type));
    }
    
    query += ' ORDER BY created_at DESC';
    
    const stmt = db.prepare(query);
    const events = stmt.all(...params);
    res.json(events);
  } catch (error) {
    console.error('Error fetching events:', error);
    res.status(500).json({ error: 'Failed to fetch events' });
  }
});

// 启动服务器
app.listen(PORT, () => {
  console.log(`Server is running on http://localhost:${PORT}`);
});
