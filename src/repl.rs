use anyhow::Result;

use crate::session::Session;

pub async fn repl(session: &mut Session) -> Result<()> {
    async fn handle_line(session: &mut Session, line: String) -> Result<()> {
        let stmts =
            sqlparser::parser::Parser::parse_sql(&sqlparser::dialect::GenericDialect {}, &line)?;
        for stmt in stmts {
            session.handle(stmt).await?;
        }
        Ok(())
    }

    let mut rl = rustyline::DefaultEditor::new()?;

    loop {
        let line = rl.readline(&if let Some(db_name) = session.current_db_name() {
            format!("{}> ", db_name)
        } else {
            "> ".to_string()
        })?;

        if let Err(e) = handle_line(session, line).await {
            tracing::error!("{:#}", e)
        }
    }
}
