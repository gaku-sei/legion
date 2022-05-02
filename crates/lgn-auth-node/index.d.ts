/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export interface UserInfo {
  sub: string
  name?: string
  givenName?: string
  familyName?: string
  middleName?: string
  nickname?: string
  username?: string
  preferredUsername?: string
  profile?: string
  picture?: string
  website?: string
  email?: string
  emailVerified?: boolean
  gender?: string
  birthdate?: string
  zoneinfo?: string
  locale?: string
  phoneNumber?: string
  phoneNumberVerified?: boolean
  updatedAt?: string
  azureOid?: string
  azureTid?: string
}
/**
 * Init the global OAuth client.
 *
 * ## Errors
 *
 * An error occurs if:
 *   - The project directory cannot be found (optional)
 *   - The OAuth client cannot be built
 *   - The global OAuth client cannot be saved
 */
export function initOauthClient(application: string, issuerUrl: string, clientId: string, redirectUri: string): Promise<void>
/**
 * Authenticate the user.
 *
 * ## Errors
 *
 * An error occurs if:
 *   - The global OAuth client is not found
 *   - The user cannot be authenticated
 */
export function authenticate(scopes: Array<string>, extraParams?: Record<string, string> | undefined | null): Promise<UserInfo>
/**
 * Returns the current access token.
 *
 * ## Errors
 *
 * An error occurs if:
 *   - The global OAuth client is not found
 *   - The access token couldn't be read from disk
 */
export function getAccessToken(): string
