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

const PAGE_SIZE = 20;

// API 路由：获取 validators（分页 + address 过滤）
app.get('/api/validators', (req, res) => {
  try {
    const { page = 1, address } = req.query;
    const pageNum = Math.max(1, parseInt(page, 10) || 1);
    const offset = (pageNum - 1) * PAGE_SIZE;

    let where = 'WHERE 1=1';
    const params = [];
    if (address && String(address).trim()) {
      where += ' AND address LIKE ?';
      params.push('%' + String(address).trim() + '%');
    }

    const countStmt = db.prepare(`SELECT COUNT(*) as total FROM validators ${where}`);
    const { total } = countStmt.get(...params);

    const dataStmt = db.prepare(
      `SELECT address, pubkey FROM validators ${where} ORDER BY created_at DESC LIMIT ? OFFSET ?`
    );
    const validators = dataStmt.all(...params, PAGE_SIZE, offset);

    res.json({ data: validators, total: total, page: pageNum, pageSize: PAGE_SIZE });
  } catch (error) {
    console.error('Error fetching validators:', error);
    res.status(500).json({ error: 'Failed to fetch validators' });
  }
});

// 按 delegator_address 查询 events，并按 validator_address 分组为 [{ validator_address, events: [{ transaction_hash, event_type, amount, shares }] }]
function getEventsGroupedByValidator(db, delegatorAddress) {
  const stmt = db.prepare(
    `SELECT validator_address, transaction_hash, event_type, amount, shares
     FROM events WHERE delegator_address = ? ORDER BY validator_address`
  );
  const rows = stmt.all(delegatorAddress);
  const byValidator = new Map();
  for (const row of rows) {
    const v = row.validator_address;
    if (!byValidator.has(v)) byValidator.set(v, []);
    byValidator.get(v).push({
      transaction_hash: row.transaction_hash || null,
      event_type: row.event_type,
      amount: row.amount,
      shares: row.shares,
    });
  }
  return Array.from(byValidator.entries()).map(([validator_address, events]) => ({
    validator_address,
    events,
  }));
}

// API 路由：获取 delegators（分页 + address 过滤，可选 with_events 带回 events 分组数据）
app.get('/api/delegators', (req, res) => {
  try {
    const { page = 1, address, with_events } = req.query;
    const pageNum = Math.max(1, parseInt(page, 10) || 1);
    const offset = (pageNum - 1) * PAGE_SIZE;

    let where = 'WHERE 1=1';
    const params = [];
    if (address && String(address).trim()) {
      where += ' AND address LIKE ?';
      params.push('%' + String(address).trim() + '%');
    }

    const countStmt = db.prepare(`SELECT COUNT(*) as total FROM delegators ${where}`);
    const { total } = countStmt.get(...params);

    const dataStmt = db.prepare(
      `SELECT address, total_delegated, total_undelegated FROM delegators ${where} ORDER BY created_at DESC LIMIT ? OFFSET ?`
    );
    const delegators = dataStmt.all(...params, PAGE_SIZE, offset);

    if (with_events === '1' || with_events === 'true') {
      delegators.forEach((d) => {
        d.data = getEventsGroupedByValidator(db, d.address);
      });
    }

    res.json({ data: delegators, total: total, page: pageNum, pageSize: PAGE_SIZE });
  } catch (error) {
    console.error('Error fetching delegators:', error);
    res.status(500).json({ error: 'Failed to fetch delegators' });
  }
});

// API 路由：获取 events（分页 + 过滤，transaction_hash 第一列）
app.get('/api/events', (req, res) => {
  try {
    const { page = 1, delegator_address, validator_address, event_type } = req.query;
    const pageNum = Math.max(1, parseInt(page, 10) || 1);
    const offset = (pageNum - 1) * PAGE_SIZE;

    let where = 'WHERE 1=1';
    const params = [];

    if (delegator_address && String(delegator_address).trim()) {
      where += ' AND delegator_address = ?';
      params.push(String(delegator_address).trim());
    }
    if (validator_address && String(validator_address).trim()) {
      where += ' AND validator_address = ?';
      params.push(String(validator_address).trim());
    }
    if (event_type !== undefined && event_type !== '') {
      where += ' AND event_type = ?';
      params.push(parseInt(event_type, 10));
    }

    const countStmt = db.prepare(`SELECT COUNT(*) as total FROM events ${where}`);
    const { total } = countStmt.get(...params);

    const dataStmt = db.prepare(
      `SELECT transaction_hash, delegator_address, validator_address, event_type, amount, shares, block_number, block_timestamp FROM events ${where} ORDER BY created_at DESC LIMIT ? OFFSET ?`
    );
    const events = dataStmt.all(...params, PAGE_SIZE, offset);

    res.json({ data: events, total: total, page: pageNum, pageSize: PAGE_SIZE });
  } catch (error) {
    console.error('Error fetching events:', error);
    res.status(500).json({ error: 'Failed to fetch events' });
  }
});

// 启动服务器
app.listen(PORT, () => {
  console.log(`Server is running on http://0.0.0.0:${PORT}`);
});
