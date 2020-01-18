use crate::conduit::{articles, favorites, followers, users};
use crate::db::models::{Article, NewArticle};
use crate::db::Repo;
use crate::domain;
use crate::domain::repositories::{ArticleRepository, UsersRepository};
use crate::domain::{DatabaseError, GetUserError};
use diesel::PgConnection;
use uuid::Uuid;

pub struct Repository<'a>(pub &'a Repo<PgConnection>);

impl<'a> ArticleRepository for Repository<'a> {
    fn publish(
        &self,
        draft: domain::ArticleContent,
        author: &domain::User,
    ) -> Result<domain::Article, domain::PublishArticleError> {
        let result: Article = articles::insert(&self.0, NewArticle::from((&draft, author)))?;

        let metadata = domain::ArticleMetadata::new(result.created_at, result.updated_at);
        let slug = draft.slug();
        let article = domain::Article::new(draft, slug, author.profile.clone(), metadata, 0);
        Ok(article)
    }

    fn get_by_slug(&self, slug: &str) -> Result<domain::Article, domain::GetArticleError> {
        Ok(articles::find_one(&self.0, &slug)?)
    }

    fn get_article_view(
        &self,
        viewer: &domain::User,
        article: domain::Article,
    ) -> Result<domain::ArticleView, domain::GetArticleError> {
        let author_view = self.get_view(viewer, &article.author.username).unwrap();
        let is_favorite = favorites::is_favorite(&self.0, viewer.id, &article.slug)?;
        let article_view = domain::ArticleView {
            content: article.content,
            slug: article.slug,
            author: author_view,
            metadata: article.metadata,
            favorited: is_favorite,
            favorites_count: article.favorites_count,
            viewer: viewer.id,
        };
        Ok(article_view)
    }

    fn get_articles_views(
        &self,
        viewer: &domain::User,
        articles: Vec<domain::Article>,
    ) -> Result<Vec<domain::ArticleView>, DatabaseError> {
        let slugs: Vec<String> = articles.iter().map(|a| a.slug.to_owned()).collect();
        let slugs: Vec<&str> = slugs.iter().map(|slug| slug.as_str()).collect();

        let favs = favorites::are_favorite(&self.0, viewer.id, slugs)?;
        articles
            .into_iter()
            .map(|a| {
                let favorited = favs[a.slug.as_str()];
                let author_view = self.get_view(viewer, &a.author.username)?;
                let article_view = domain::ArticleView {
                    content: a.content,
                    slug: a.slug,
                    author: author_view,
                    metadata: a.metadata,
                    favorited,
                    favorites_count: a.favorites_count,
                    viewer: viewer.id,
                };
                Ok(article_view)
            })
            .collect()
    }

    fn find_articles(
        &self,
        query: domain::ArticleQuery,
    ) -> Result<Vec<domain::Article>, DatabaseError> {
        let result: Vec<domain::Article> = articles::find(&self.0, query)?
            .into_iter()
            .map(|a| a.into())
            .collect();
        Ok(result)
    }

    fn delete_article(&self, article: &domain::Article) -> Result<(), DatabaseError> {
        Ok(articles::delete(&self.0, &article.slug)?)
    }

    fn update_article(
        &self,
        article: domain::Article,
        update: domain::ArticleUpdate,
    ) -> Result<domain::Article, DatabaseError> {
        articles::update(&self.0, (&update).into(), &article.slug)?;
        Ok(self.get_by_slug(&article.slug)?)
    }

    fn favorite(
        &self,
        article: &domain::Article,
        user: &domain::User,
    ) -> Result<domain::FavoriteOutcome, domain::DatabaseError> {
        favorites::favorite(&self.0, user.id, &article.slug)
    }

    fn unfavorite(
        &self,
        article: &domain::Article,
        user: &domain::User,
    ) -> Result<domain::UnfavoriteOutcome, domain::DatabaseError> {
        favorites::unfavorite(&self.0, user.id, &article.slug)
    }
}

impl<'a> UsersRepository for Repository<'a> {
    fn get_by_id(&self, user_id: Uuid) -> Result<domain::User, GetUserError> {
        let u = users::find(&self.0, user_id)?;
        let profile = domain::Profile::new(u.username, u.bio, u.image);
        Ok(domain::User::new(u.id, u.email, profile))
    }

    fn get_view(
        &self,
        viewer: &domain::User,
        username: &str,
    ) -> Result<domain::ProfileView, GetUserError> {
        let viewed_user = users::find_by_username(&self.0, username)?;
        let following = followers::is_following(&self.0, viewer.id, viewed_user.id)?;
        let view = domain::ProfileView {
            profile: domain::Profile {
                username: viewed_user.username,
                bio: viewed_user.bio,
                image: viewed_user.image,
            },
            following,
            viewer: viewer.id,
        };
        Ok(view)
    }
}
