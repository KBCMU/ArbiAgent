# ArbiAgent Deployment

## Backend (Render)

- **URL:** https://arbiagent-backend.onrender.com
- **Service:** ArbiAgent-Backend (`srv-d6ivhhjh46gs73ad69jg`)
- **Config:** rootDir=`backend-rust`, startCommand=`./target/release/arbiagent-backend`
- **Env vars:** Set via Render dashboard or `update_environment_variables` MCP (DOME_API_KEY, SUPABASE_*, DATABASE_URL, PORT)

## Frontend (Vercel)

- **Project:** arbi-agent
- **Domains:** arbi-agent.vercel.app, arbi-agent-kbcmus-projects.vercel.app

### Required: Set Root Directory

1. Go to [Vercel Dashboard](https://vercel.com/kbcmus-projects/arbi-agent) → Settings → General
2. Under **Root Directory**, set to `frontend`
3. Save and redeploy (or push to `main` to trigger auto-deploy)

### Required: Environment Variables

In Vercel → Settings → Environment Variables, add:

| Name | Value | Environment |
|------|-------|-------------|
| `NEXT_PUBLIC_API_URL` | `https://arbiagent-backend.onrender.com` | Production, Preview |
| `NEXT_PUBLIC_SUPABASE_URL` | (from Supabase project) | Production, Preview |
| `NEXT_PUBLIC_SUPABASE_ANON_KEY` | (from Supabase project) | Production, Preview |

### Optional: Update via API

```bash
# With VERCEL_TOKEN set:
curl -X PATCH "https://api.vercel.com/v9/projects/arbi-agent?teamId=team_x1I6Lcg2v8DrH0YRmVnPmIqu" \
  -H "Authorization: Bearer $VERCEL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"rootDirectory": "frontend"}'
```
