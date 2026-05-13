use kakaotalk::{Content, KakaoTalk};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let kt = KakaoTalk::new();

    let auth = kt.authorization().await?;
    println!("{}", auth);

    let user = kt.user().await?;
    println!("{}", user.email);
    println!("{}", user.name);
    println!("{}", user.profile_url);

    let results = kt.search_channels("카페").await?;
    for ch in results.values() {
        if !ch.is_chattable() {
            println!("{} 채널이 1:1 문의를 지원하지 않습니다.", ch.title);
            continue;
        }

        let is_friend = ch.add().await.unwrap_or(false);
        if !is_friend {
            println!("{} 채널 추가에 실패 하였습니다.", ch.title);
            continue;
        } else {
            println!("{} 채널 추가에 성공 하였습니다.", ch.title);
        }

        let ok = ch.send(&[Content::text("안녕하세요")]);
        println!("{}", ok);
        println!();
    }
    println!("총 {}개", results.len());
    Ok(())
}
