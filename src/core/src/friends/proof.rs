use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Central,
    Peripheral,
}
impl Role {
    fn byte(self) -> u8 {
        match self {
            Self::Central => 0,
            Self::Peripheral => 1,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Observation {
    pub response_age_ms: Option<u64>,
    pub fresh: bool,
    pub session_authenticated: bool,
}
pub(super) struct Session {
    content_key_sent: bool,
    link: LinkHandle,
    local: [u8; 54],
    remote: Option<[u8; 54]>,
    born: u64,
    hello_sent: bool,
    ready: Option<u64>,
    local_proof: Option<[u8; 66]>,
    transmissions: u8,
    candidate: Option<[u8; 66]>,
    attempted: bool,
    verified: Option<u64>,
}
impl Session {
    fn transcript(&self, role: u8) -> Vec<u8> {
        let remote = self.remote.as_ref().unwrap();
        let (central, peripheral) = if self.local[1] == 0 {
            (&self.local, remote)
        } else {
            (remote, &self.local)
        };
        let mut bytes = b"meshfest/link-proof/v1\0".to_vec();
        bytes.push(role);
        bytes.extend_from_slice(central);
        bytes.extend_from_slice(peripheral);
        bytes
    }
    fn authenticated(&self) -> bool {
        self.verified.is_some() && self.transmissions > 0
    }
    fn order(&self) -> ([u8; 32], [u8; 16], [u8; 16]) {
        let remote = self.remote.as_ref().unwrap();
        let (central, peripheral) = if self.local[1] == 0 {
            (&self.local, remote)
        } else {
            (remote, &self.local)
        };
        (
            central[22..].try_into().unwrap(),
            central[6..22].try_into().unwrap(),
            peripheral[6..22].try_into().unwrap(),
        )
    }
}
impl Friends {
    pub fn effective_capacities(&self, link: &LinkHandle) -> Result<(u16, u16), Error> {
        let s = &self.sessions[self.session(link)?];
        let remote = s.remote.as_ref().ok_or(Error::Stale)?;
        let word = |bytes: &[u8]| u16::from_be_bytes(bytes.try_into().unwrap());
        Ok((
            word(&s.local[2..4]).min(word(&remote[4..6])),
            word(&s.local[4..6]).min(word(&remote[2..4])),
        ))
    }
    pub(super) fn receive_capacity(&self, link: &LinkHandle) -> Result<usize, Error> {
        let s = &self.sessions[self.session(link)?];
        Ok(if s.remote.is_some() {
            usize::from(self.effective_capacities(link)?.1)
        } else {
            usize::from(u16::from_be_bytes(s.local[4..6].try_into().unwrap()))
        })
    }
    /// Native admission supplies physical role/limits and a fallible OS CSPRNG
    /// callback. It must retain its node/address connection-admission gate.
    /// This method constructs the exact local HELLO and registers bounded intake.
    pub fn start_link(
        &mut self,
        ingress: &mut Ingress,
        link: LinkHandle,
        role: Role,
        capacities: (u16, u16),
        now: u64,
        random: impl FnOnce() -> Result<[u8; 16], Error>,
    ) -> Result<[u8; 54], Error> {
        self.advance(now)?;
        if self.sessions.len() >= 8 || self.sessions.iter().any(|s| s.link == link) {
            return Err(Error::Full);
        }
        if !(146..=512).contains(&capacities.0) || !(146..=512).contains(&capacities.1) {
            return Err(Error::Invalid);
        }
        let nonce = random()?;
        if nonce == [0; 16] || self.sessions.iter().any(|s| s.local[6..22] == nonce) {
            return Err(Error::Invalid);
        }
        let mut local = [0; 54];
        local[0] = 1;
        local[1] = role.byte();
        local[2..4].copy_from_slice(&capacities.0.to_be_bytes());
        local[4..6].copy_from_slice(&capacities.1.to_be_bytes());
        local[6..22].copy_from_slice(&nonce);
        local[22..].copy_from_slice(&self.own_ed);
        ingress.register(&link, usize::from(capacities.1), now)?;
        self.sessions.push(Session {
            content_key_sent: false,
            link,
            local,
            remote: None,
            born: now,
            hello_sent: false,
            ready: None,
            local_proof: None,
            transmissions: 0,
            candidate: None,
            attempted: false,
            verified: None,
        });
        Ok(local)
    }
    fn session(&self, link: &LinkHandle) -> Result<usize, Error> {
        self.sessions
            .iter()
            .position(|s| s.link == *link)
            .ok_or(Error::Stale)
    }
    pub fn hello_transmitted(&mut self, link: &LinkHandle, now: u64) -> Result<(), Error> {
        self.advance(now)?;
        let i = self.session(link)?;
        let s = &mut self.sessions[i];
        if now > s.born + 10_000 {
            return Err(Error::Stale);
        }
        s.hello_sent = true;
        if s.remote.is_some() && s.ready.is_none() {
            s.ready = Some(now);
        }
        Ok(())
    }
    pub(super) fn link_ready(&self, link: &LinkHandle) -> bool {
        self.sessions
            .iter()
            .any(|s| s.link == *link && s.ready.is_some())
    }
    pub(super) fn control(
        &mut self,
        ingress: &mut Ingress,
        link: &LinkHandle,
        kind: u8,
        body: &[u8],
        now: u64,
    ) -> Result<(), Error> {
        self.advance(now)?;
        let i = self.session(link)?;
        if kind == 2 {
            let hello: [u8; 54] = body.try_into().map_err(|_| Error::Invalid)?;
            let s = &mut self.sessions[i];
            if let Some(old) = s.remote {
                if old == hello {
                    return Ok(());
                }
                self.sessions.remove(i);
                ingress.disconnect(link, now)?;
                return Err(Error::Stale);
            }
            if now > s.born + 10_000 || hello[1] == s.local[1] || hello[22..] == self.own_ed {
                return Err(Error::Invalid);
            }
            public_key(&hello[22..])?;
            s.remote = Some(hello);
            if s.hello_sent {
                s.ready = Some(now);
            }
            return Ok(());
        }
        let s = &mut self.sessions[i];
        let candidate: [u8; 66] = body.try_into().map_err(|_| Error::Invalid)?;
        if s.candidate == Some(candidate) && s.attempted {
            return Ok(());
        }
        if s.ready.is_none_or(|at| now > at + 10_000) {
            return Err(Error::Stale);
        }
        if let Some(old) = s.candidate {
            if old != candidate {
                return Err(Error::Invalid);
            }
        } else {
            s.candidate = Some(candidate);
        }
        if s.attempted {
            return Ok(());
        }
        let Some(permit) = ingress.begin_work(link, 1, now)? else {
            return Ok(());
        };
        s.attempted = true;
        let key = s.remote.unwrap()[22..].try_into().unwrap();
        let valid = candidate[1] != s.local[1]
            && verify(&key, &s.transcript(candidate[1]), &candidate[2..]);
        ingress.finish_work(permit)?;
        if valid {
            s.verified = Some(now);
            self.remember_response(i);
        }
        Ok(())
    }
    pub fn local_proof(
        &mut self,
        ingress: &mut Ingress,
        provider: &IdentityKeySession,
        link: &LinkHandle,
        now: u64,
    ) -> Result<Option<[u8; 66]>, Error> {
        self.advance(now)?;
        if !provider.matches_generation(&self.generation) {
            self.clear_session_trust();
            return Err(Error::Stale);
        }
        let i = self.session(link)?;
        let s = &mut self.sessions[i];
        if s.ready.is_none_or(|at| now > at + 10_000) {
            return Err(Error::Stale);
        }
        if let Some(proof) = s.local_proof {
            return Ok((s.transmissions < 2).then_some(proof));
        }
        let Some(permit) = ingress.begin_work(link, 1, now)? else {
            return Ok(None);
        };
        let signed = provider.sign(s.transcript(s.local[1]));
        ingress.finish_work(permit)?;
        let signature = signed.map_err(|_| Error::Provider)?;
        let mut proof = [0; 66];
        proof[0] = 1;
        proof[1] = s.local[1];
        proof[2..].copy_from_slice(&signature);
        s.local_proof = Some(proof);
        Ok(Some(proof))
    }
    /// Call only after the native send accepted these exact local proof bytes.
    pub fn proof_transmitted(
        &mut self,
        link: &LinkHandle,
        proof: &[u8; 66],
        now: u64,
    ) -> Result<(), Error> {
        self.advance(now)?;
        let i = self.session(link)?;
        let s = &mut self.sessions[i];
        if s.local_proof.as_ref() != Some(proof)
            || s.transmissions >= 2
            || s.ready.is_none_or(|at| now > at + 10_000)
        {
            return Err(Error::Stale);
        }
        s.transmissions += 1;
        self.remember_response(i);
        Ok(())
    }
    fn remember_response(&mut self, i: usize) {
        let s = &self.sessions[i];
        if !s.authenticated() {
            return;
        }
        let key = s.remote.unwrap()[22..].try_into().unwrap();
        let at = s.verified.unwrap();
        if let Some(pin) = self.pinned(&key) {
            if let Some(old) = self
                .last_responses
                .iter_mut()
                .find(|(t, _)| *t == pin.token)
            {
                old.1 = old.1.max(at);
            } else if self.last_responses.len() < 128 {
                self.last_responses.push((pin.token, at));
            }
        }
    }
    pub fn observation(&mut self, token: &SendToken, now: u64) -> Result<Observation, Error> {
        self.advance(now)?;
        let pin = self.current(token)?;
        if pin.replacing {
            return Err(Error::Stale);
        }
        let age = self
            .last_responses
            .iter()
            .find(|(t, _)| *t == *token)
            .and_then(|(_, at)| now.checked_sub(*at));
        let matching: Vec<_> = self
            .sessions
            .iter()
            .filter(|s| s.authenticated() && s.remote.is_some_and(|r| r[22..] == token.tuple[..32]))
            .collect();
        Ok(Observation {
            response_age_ms: age,
            fresh: matching.iter().any(|s| now < s.born + 60_000),
            session_authenticated: !matching.is_empty(),
        })
    }
    pub fn disconnect(
        &mut self,
        ingress: &mut Ingress,
        link: &LinkHandle,
        now: u64,
    ) -> Result<(), Error> {
        self.advance(now)?;
        self.sessions.retain(|s| s.link != *link);
        ingress.disconnect(link, now)?;
        Ok(())
    }
    /// Only independently authenticated duplicates participate. A sole link or
    /// unproved peer claim never causes an eviction recommendation.
    pub fn duplicate_links_to_close(&self) -> Vec<LinkHandle> {
        self.sessions
            .iter()
            .filter(|s| {
                s.authenticated()
                    && self.sessions.iter().any(|other| {
                        other.authenticated()
                            && other.remote.unwrap()[22..] == s.remote.unwrap()[22..]
                            && other.order() < s.order()
                    })
            })
            .map(|s| s.link.clone())
            .collect()
    }
}

impl Friends {
    /// Caller selects the v1 signing policy. Only CHAT/ANNOUNCE from this loaded
    /// identity may be signed; first actual send per link includes the full key.
    pub fn sign_content(
        &mut self,
        ingress: &mut Ingress,
        provider: &IdentityKeySession,
        link: &LinkHandle,
        unsigned: &[u8],
        now: u64,
    ) -> Result<Option<SignedPacket>, Error> {
        self.advance(now)?;
        if !provider.matches_generation(&self.generation) {
            self.clear_session_trust();
            return Err(Error::Stale);
        }
        let index = self.session(link)?;
        if !self.link_ready(link) {
            return Err(Error::Stale);
        }
        let packet = codec::parse(unsigned, codec::Context::Live).map_err(|_| Error::Invalid)?;
        let header = packet.header();
        if ![1, 2].contains(&header.kind)
            || header.flags != 0
            || header.sender_id != hint(&self.own_ed)
            || header.channel_id == [0x0e, 0x9d, 0x09, 0x74]
        {
            return Err(Error::Invalid);
        }
        let includes_key = header.kind == 2 || !self.sessions[index].content_key_sent;
        let mut raw = unsigned.to_vec();
        raw[2] = 2;
        raw.extend_from_slice(&hint(&self.own_ed));
        raw.push(u8::from(includes_key));
        if includes_key {
            raw.extend_from_slice(&self.own_ed);
        }
        raw.extend_from_slice(&[0; 64]);
        let length = (raw.len() - 26) as u16;
        raw[24..26].copy_from_slice(&length.to_be_bytes());
        codec::parse(&raw, codec::Context::Live).map_err(|_| Error::Invalid)?;
        let Some(permit) = ingress.begin_work(link, 1, now)? else {
            return Ok(None);
        };
        let signature = provider.sign(transcript(&raw));
        ingress.finish_work(permit)?;
        let signature = signature.map_err(|_| Error::Provider)?;
        let at = raw.len() - 64;
        raw[at..].copy_from_slice(&signature);
        Ok(Some(SignedPacket {
            raw,
            link: link.clone(),
            generation: self.generation,
            includes_key,
        }))
    }
    pub fn content_transmitted(&mut self, packet: &SignedPacket) -> Result<(), Error> {
        if packet.generation != self.generation {
            return Err(Error::Stale);
        }
        let index = self.session(&packet.link)?;
        if packet.includes_key {
            self.sessions[index].content_key_sent = true;
        }
        Ok(())
    }
}
