use crate::{models::user::*, sqlx::store::PgOnDemandStore, Error};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserStore: Sync + Send {
    async fn add_user(&self, new_user: NewUserRow) -> Result<UserRow, Error>;

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<UserRow>, Error>;

    async fn find_user_by_firebase_uid(
        &self,
        firebase_uid: String,
    ) -> Result<Option<UserRow>, Error>;

    async fn list_users(
        &self,
        page_size: i64,
        page_token: Option<UserPageToken>,
    ) -> Result<Vec<UserRow>, Error>;

    async fn count_users(&self) -> Result<i64, Error>;

    async fn list_and_count_users(
        &self,
        page_size: i64,
        page_token: Option<UserPageToken>,
    ) -> Result<(Vec<UserRow>, i64), Error> {
        let list_fut = self.list_users(page_size, page_token);
        let count_fut = self.count_users();
        futures::try_join!(list_fut, count_fut)
    }
}

#[async_trait]
impl UserStore for PgOnDemandStore {
    async fn add_user(&self, new_user: NewUserRow) -> Result<UserRow, Error> {
        Ok(sqlx::query_file_as!(
            UserRow,
            "queries/user/insert.sql",
            &new_user.create_time,
            &new_user.update_time,
            &new_user.firebase_uid,
            &new_user.firebase_email,
            &new_user.stripe_customer_id,
        )
        .fetch_one(&*self.pool)
        .await?)
    }

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<UserRow>, Error> {
        Ok(
            sqlx::query_file_as!(UserRow, "queries/user/find_by_id.sql", &user_id,)
                .fetch_optional(&*self.pool)
                .await?,
        )
    }

    async fn find_user_by_firebase_uid(
        &self,
        firebase_uid: String,
    ) -> Result<Option<UserRow>, Error> {
        Ok(sqlx::query_file_as!(
            UserRow,
            "queries/user/find_by_firebase_uid.sql",
            &firebase_uid,
        )
        .fetch_optional(&*self.pool)
        .await?)
    }

    async fn list_users(
        &self,
        page_size: i64,
        page_token: Option<UserPageToken>,
    ) -> Result<Vec<UserRow>, Error> {
        let rows = match page_token {
            Some(page_token) => {
                // Query by page token:
                sqlx::query_file_as!(
                    UserRow,
                    "queries/user/list_at_page.sql",
                    page_token.create_time,
                    page_token.user_id,
                    page_size,
                )
                .fetch_all(&*self.pool)
                .await?
            }
            None => {
                // Query first page:
                sqlx::query_file_as!(UserRow, "queries/user/list.sql", page_size)
                    .fetch_all(&*self.pool)
                    .await?
            }
        };
        Ok(rows)
    }

    async fn count_users(&self) -> Result<i64, Error> {
        Ok(sqlx::query_file!("queries/user/count.sql")
            .fetch_one(&*self.pool)
            .await?
            .count)
    }
}
