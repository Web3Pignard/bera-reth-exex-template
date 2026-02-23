# Staking Indexer Web Server

这是一个简单的 Web 服务，用于查看 staking indexer 数据库中的数据。

## 功能

- 展示 Validators 列表（address, pubkey）
- 展示 Delegators 列表（address, total_delegated, total_undelegated）
- 展示 Events 列表（支持按 delegator_address、validator_address、event_type 过滤）

## 安装

```bash
cd web-server
npm install
```

## 运行

```bash
# 使用默认数据库路径 /data/staking_indexer.db
npm start

# 或指定自定义数据库路径
DB_PATH=/path/to/your/database.db npm start

# 或指定自定义端口
PORT=8080 npm start
```

## 访问

启动后访问 http://localhost:3000

## 环境变量

- `DB_PATH`: 数据库文件路径（默认: `/data/staking_indexer.db`）
- `PORT`: 服务器端口（默认: `3000`）
